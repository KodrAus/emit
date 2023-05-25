use crate::{empty::Empty, event::Event, props::Props};

pub trait Filter {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool;

    fn and<U>(self, other: U) -> And<Self, U>
    where
        Self: Sized,
    {
        And {
            lhs: self,
            rhs: other,
        }
    }

    fn or<U>(self, other: U) -> Or<Self, U>
    where
        Self: Sized,
    {
        Or {
            lhs: self,
            rhs: other,
        }
    }

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }
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

pub struct And<T, U> {
    lhs: T,
    rhs: U,
}

impl<T: Filter, U: Filter> Filter for And<T, U> {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        self.lhs.matches_event(evt) && self.rhs.matches_event(evt)
    }
}

pub struct Or<T, U> {
    lhs: T,
    rhs: U,
}

impl<T: Filter, U: Filter> Filter for Or<T, U> {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        self.lhs.matches_event(evt) || self.rhs.matches_event(evt)
    }
}

pub struct ByRef<'a, T: ?Sized>(&'a T);

impl<'a, T: Filter + ?Sized> Filter for ByRef<'a, T> {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        self.0.matches_event(evt)
    }
}

mod internal {
    use crate::event::Event;

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
