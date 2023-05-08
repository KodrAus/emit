use crate::{Event, Props};

pub use crate::adapt::{ByRef, Chain, Empty};

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

impl<'a, T: Target + ?Sized> Target for &'a T {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        (**self).emit_event(evt)
    }
}

#[cfg(feature = "std")]
impl<'a, T: Target + ?Sized + 'a> Target for Box<T> {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        (**self).emit_event(evt)
    }
}

impl<T: Target> Target for Option<T> {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        match self {
            Some(target) => target.emit_event(evt),
            None => Empty.emit_event(evt),
        }
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

impl Target for Empty {
    fn emit_event<P: Props>(&self, _: &Event<P>) {}
}

impl Target for fn(&Event) {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        (self)(&evt.erase())
    }
}

pub struct FromFn<F>(F);

impl<F: Fn(&Event)> Target for FromFn<F> {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        (self.0)(&evt.erase())
    }
}

pub fn from_fn<F: Fn(&Event)>(f: F) -> FromFn<F> {
    FromFn(f)
}

mod internal {
    use crate::Event;

    pub trait DispatchTarget {
        fn dispatch_emit_to(&self, evt: &Event);
    }

    pub trait SealedTarget {
        fn erase_to(&self) -> crate::internal::Erased<&dyn DispatchTarget>;
    }
}

pub trait ErasedTarget: internal::SealedTarget {}

impl<T: Target> ErasedTarget for T {}

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

impl<'a> Target for dyn ErasedTarget + 'a {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        self.erase_to().0.dispatch_emit_to(&evt.erase())
    }
}

impl<'a> Target for dyn ErasedTarget + Send + Sync + 'a {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        (self as &(dyn ErasedTarget + 'a)).emit_event(evt)
    }
}
