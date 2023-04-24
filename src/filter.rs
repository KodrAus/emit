use crate::{Event, Props};

pub trait Filter {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool;

    fn chain<U: Filter>(self, other: U) -> Chain<Self, U>
    where
        Self: Sized,
    {
        Chain {
            first: self,
            second: other,
        }
    }

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }
}

impl<F: Fn(&Event) -> bool> Filter for F {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        (self)(&evt.erase())
    }
}

impl<T: Filter, U: Filter> Filter for Chain<T, U> {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        self.first.matches_event(evt) && self.second.matches_event(evt)
    }
}

impl<'a, C: Filter + ?Sized> Filter for ByRef<'a, C> {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        self.0.matches_event(evt)
    }
}

pub(crate) struct Always;

impl Filter for Always {
    fn matches_event<P: Props>(&self, _: &Event<P>) -> bool {
        true
    }
}

pub fn default() -> impl Filter {
    Always
}

pub struct Chain<T, U> {
    pub(crate) first: T,
    pub(crate) second: U,
}

pub struct ByRef<'a, T: ?Sized>(pub(crate) &'a T);

mod internal {
    use crate::Event;

    pub trait DispatchFilter {
        fn dispatch_emit_when(&self, evt: &Event) -> bool;
    }

    pub trait SealedFilter {
        fn erase_when(&self) -> crate::internal::Erased<&dyn DispatchFilter>;
    }
}

pub trait ErasedWhen: internal::SealedFilter {}

impl<T: Filter> ErasedWhen for T {}

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

impl<'a> Filter for dyn ErasedWhen + 'a {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        self.erase_when().0.dispatch_emit_when(&evt.erase())
    }
}
