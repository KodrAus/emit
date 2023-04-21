use crate::{event::Event, props::Props};

pub trait To {
    fn emit_to<P: Props>(&self, evt: &Event<P>);

    fn chain<U: To>(self, other: U) -> Chain<Self, U>
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

impl<F: Fn(Event)> To for F {
    fn emit_to<P: Props>(&self, evt: &Event<P>) {
        (self)(evt.erase())
    }
}

impl<T: To, U: To> To for Chain<T, U> {
    fn emit_to<P: Props>(&self, evt: &Event<P>) {
        self.first.emit_to(evt);
        self.second.emit_to(evt);
    }
}

impl<'a, T: To + ?Sized> To for ByRef<'a, T> {
    fn emit_to<P: Props>(&self, evt: &Event<P>) {
        self.0.emit_to(evt)
    }
}

pub(crate) struct Discard;

impl To for Discard {
    fn emit_to<P: Props>(&self, _: &Event<P>) {}
}

pub fn default() -> impl To {
    Discard
}

pub struct Chain<T, U> {
    pub(crate) first: T,
    pub(crate) second: U,
}

pub struct ByRef<'a, T: ?Sized>(pub(crate) &'a T);

mod internal {
    use crate::Event;

    pub trait DispatchTo {
        fn dispatch_emit_to(&self, evt: &Event);
    }

    pub trait SealedTo {
        fn erase_to(&self) -> crate::internal::Erased<&dyn DispatchTo>;
    }
}

pub trait ErasedTo: internal::SealedTo {}

impl<T: To> ErasedTo for T {}

impl<T: To> internal::SealedTo for T {
    fn erase_to(&self) -> crate::internal::Erased<&dyn internal::DispatchTo> {
        crate::internal::Erased(self)
    }
}

impl<T: To> internal::DispatchTo for T {
    fn dispatch_emit_to(&self, evt: &Event) {
        self.emit_to(evt)
    }
}

impl<'a> To for dyn ErasedTo + 'a {
    fn emit_to<P: Props>(&self, evt: &Event<P>) {
        self.erase_to().0.dispatch_emit_to(&evt.erase())
    }
}
