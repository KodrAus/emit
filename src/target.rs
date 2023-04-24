use crate::{event::Event, props::Props};

pub trait Target {
    fn emit_event<P: Props>(&self, evt: &Event<P>);

    fn chain<U: Target>(self, other: U) -> Chain<Self, U>
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

impl<F: Fn(Event)> Target for F {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        (self)(evt.erase())
    }
}

impl<T: Target, U: Target> Target for Chain<T, U> {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        self.first.emit_event(evt);
        self.second.emit_event(evt);
    }
}

impl<'a, T: Target + ?Sized> Target for ByRef<'a, T> {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        self.0.emit_event(evt)
    }
}

pub(crate) struct Discard;

impl Target for Discard {
    fn emit_event<P: Props>(&self, _: &Event<P>) {}
}

pub fn default() -> impl Target {
    Discard
}

pub struct Chain<T, U> {
    pub(crate) first: T,
    pub(crate) second: U,
}

pub struct ByRef<'a, T: ?Sized>(pub(crate) &'a T);

mod internal {
    use crate::Event;

    pub trait DispatchTarget {
        fn dispatch_emit_to(&self, evt: &Event);
    }

    pub trait SealedTarget {
        fn erase_to(&self) -> crate::internal::Erased<&dyn DispatchTarget>;
    }
}

pub trait ErasedTo: internal::SealedTarget {}

impl<T: Target> ErasedTo for T {}

impl<T: Target> internal::SealedTarget for T {
    fn erase_to(&self) -> crate::internal::Erased<&dyn internal::DispatchTarget> {
        crate::internal::Erased(self)
    }
}

impl<T: Target> internal::DispatchTarget for T {
    fn dispatch_emit_to(&self, evt: &Event) {
        self.emit_event(evt)
    }
}

impl<'a> Target for dyn ErasedTo + 'a {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        self.erase_to().0.dispatch_emit_to(&evt.erase())
    }
}
