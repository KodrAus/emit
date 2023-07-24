use core::time::Duration;

use crate::{
    empty::Empty,
    event::Event,
    props::{ErasedProps, Props},
};

pub trait Target {
    fn event<P: Props>(&self, evt: &Event<P>);

    fn blocking_flush(&self, timeout: Duration);

    fn and<U>(self, other: U) -> And<Self, U>
    where
        Self: Sized,
    {
        And {
            lhs: self,
            rhs: other,
        }
    }

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }
}

impl<'a, T: Target + ?Sized> Target for &'a T {
    fn event<P: Props>(&self, evt: &Event<P>) {
        (**self).event(evt)
    }

    fn blocking_flush(&self, timeout: Duration) {
        (**self).blocking_flush(timeout)
    }
}

#[cfg(feature = "std")]
impl<'a, T: Target + ?Sized + 'a> Target for Box<T> {
    fn event<P: Props>(&self, evt: &Event<P>) {
        (**self).event(evt)
    }

    fn blocking_flush(&self, timeout: Duration) {
        (**self).blocking_flush(timeout)
    }
}

impl<T: Target> Target for Option<T> {
    fn event<P: Props>(&self, evt: &Event<P>) {
        match self {
            Some(target) => target.event(evt),
            None => Empty.event(evt),
        }
    }

    fn blocking_flush(&self, timeout: Duration) {
        match self {
            Some(target) => target.blocking_flush(timeout),
            None => Empty.blocking_flush(timeout),
        }
    }
}

impl Target for Empty {
    fn event<P: Props>(&self, _: &Event<P>) {}
    fn blocking_flush(&self, _: Duration) {}
}

impl Target for fn(&Event<&dyn ErasedProps>) {
    fn event<P: Props>(&self, evt: &Event<P>) {
        (self)(&evt.erase())
    }

    fn blocking_flush(&self, _: Duration) {}
}

pub struct FromFn<F>(F);

impl<F: Fn(&Event<&dyn ErasedProps>)> Target for FromFn<F> {
    fn event<P: Props>(&self, evt: &Event<P>) {
        (self.0)(&evt.erase())
    }

    fn blocking_flush(&self, _: Duration) {}
}

pub fn from_fn<F: Fn(&Event<&dyn ErasedProps>)>(f: F) -> FromFn<F> {
    FromFn(f)
}

pub struct And<T, U> {
    lhs: T,
    rhs: U,
}

impl<T: Target, U: Target> Target for And<T, U> {
    fn event<P: Props>(&self, evt: &Event<P>) {
        self.lhs.event(evt);
        self.rhs.event(evt);
    }

    fn blocking_flush(&self, timeout: Duration) {
        let timeout = timeout / 2;

        self.lhs.blocking_flush(timeout);
        self.rhs.blocking_flush(timeout);
    }
}

pub struct ByRef<'a, T: ?Sized>(&'a T);

impl<'a, T: Target + ?Sized> Target for ByRef<'a, T> {
    fn event<P: Props>(&self, evt: &Event<P>) {
        self.0.event(evt)
    }

    fn blocking_flush(&self, timeout: Duration) {
        self.0.blocking_flush(timeout)
    }
}

mod internal {
    use core::time::Duration;

    use crate::{event::Event, props::ErasedProps};

    pub trait DispatchTarget {
        fn dispatch_event(&self, evt: &Event<&dyn ErasedProps>);
        fn dispatch_blocking_flush(&self, timeout: Duration);
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
    fn dispatch_event(&self, evt: &Event<&dyn ErasedProps>) {
        self.event(evt)
    }

    fn dispatch_blocking_flush(&self, timeout: Duration) {
        self.blocking_flush(timeout)
    }
}

impl<'a> Target for dyn ErasedTarget + 'a {
    fn event<P: Props>(&self, evt: &Event<P>) {
        self.erase_to().0.dispatch_event(&evt.erase())
    }

    fn blocking_flush(&self, timeout: Duration) {
        self.erase_to().0.dispatch_blocking_flush(timeout)
    }
}

impl<'a> Target for dyn ErasedTarget + Send + Sync + 'a {
    fn event<P: Props>(&self, evt: &Event<P>) {
        (self as &(dyn ErasedTarget + 'a)).event(evt)
    }

    fn blocking_flush(&self, timeout: Duration) {
        (self as &(dyn ErasedTarget + 'a)).blocking_flush(timeout)
    }
}
