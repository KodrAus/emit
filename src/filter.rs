use crate::{Event, Props};

pub use crate::empty::Empty;

pub trait Filter {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool;
}

impl<'a, F: Filter + ?Sized> Filter for &'a F {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        (**self).matches_event(evt)
    }
}

#[cfg(feature = "std")]
impl<'a, F: Filter + ?Sized + 'a> Filter for Box<F> {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        (**self).matches_event(evt)
    }
}

impl<F: Filter> Filter for Option<F> {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        match self {
            Some(filter) => filter.matches_event(evt),
            None => Empty.matches_event(evt),
        }
    }
}

impl Filter for Empty {
    fn matches_event<P: Props>(&self, _: &Event<P>) -> bool {
        true
    }
}

impl Filter for fn(&Event) -> bool {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        (self)(&evt.erase())
    }
}

pub struct FromFn<F>(F);

impl<F: Fn(&Event) -> bool> Filter for FromFn<F> {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        (self.0)(&evt.erase())
    }
}

pub fn from_fn<F: Fn(&Event)>(f: F) -> FromFn<F> {
    FromFn(f)
}

mod internal {
    use crate::Event;

    pub trait DispatchFilter {
        fn dispatch_emit_when(&self, evt: &Event) -> bool;
    }

    pub trait SealedFilter {
        fn erase_when(&self) -> crate::internal::Erased<&dyn DispatchFilter>;
    }
}

pub trait ErasedFilter: internal::SealedFilter {}

impl<T: Filter> ErasedFilter for T {}

impl<T: Filter> internal::SealedFilter for T {
    fn erase_when(&self) -> crate::internal::Erased<&dyn internal::DispatchFilter> {
        crate::internal::Erased(self)
    }
}

impl<T: Filter> internal::DispatchFilter for T {
    fn dispatch_emit_when(&self, evt: &Event) -> bool {
        self.matches_event(evt)
    }
}

impl<'a> Filter for dyn ErasedFilter + 'a {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        self.erase_when().0.dispatch_emit_when(&evt.erase())
    }
}

impl<'a> Filter for dyn ErasedFilter + Send + Sync + 'a {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        (self as &(dyn ErasedFilter + 'a)).matches_event(evt)
    }
}
