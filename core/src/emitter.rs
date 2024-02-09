use core::time::Duration;

use crate::{
    empty::Empty,
    event::Event,
    props::{ErasedProps, Props},
};

pub trait Emitter {
    fn emit<P: Props>(&self, evt: &Event<P>);

    fn blocking_flush(&self, timeout: Duration);

    fn and_to<U>(self, other: U) -> And<Self, U>
    where
        Self: Sized,
    {
        And {
            left: self,
            right: other,
        }
    }

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }
}

impl<'a, T: Emitter + ?Sized> Emitter for &'a T {
    fn emit<P: Props>(&self, evt: &Event<P>) {
        (**self).emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) {
        (**self).blocking_flush(timeout)
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Emitter + ?Sized + 'a> Emitter for alloc::boxed::Box<T> {
    fn emit<P: Props>(&self, evt: &Event<P>) {
        (**self).emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) {
        (**self).blocking_flush(timeout)
    }
}

impl<T: Emitter> Emitter for Option<T> {
    fn emit<P: Props>(&self, evt: &Event<P>) {
        match self {
            Some(target) => target.emit(evt),
            None => Empty.emit(evt),
        }
    }

    fn blocking_flush(&self, timeout: Duration) {
        match self {
            Some(target) => target.blocking_flush(timeout),
            None => Empty.blocking_flush(timeout),
        }
    }
}

impl Emitter for Empty {
    fn emit<P: Props>(&self, _: &Event<P>) {}
    fn blocking_flush(&self, _: Duration) {}
}

impl Emitter for fn(&Event<&dyn ErasedProps>) {
    fn emit<P: Props>(&self, evt: &Event<P>) {
        (self)(&evt.erase())
    }

    fn blocking_flush(&self, _: Duration) {}
}

pub struct FromFn<F>(F);

impl<F: Fn(&Event<&dyn ErasedProps>)> Emitter for FromFn<F> {
    fn emit<P: Props>(&self, evt: &Event<P>) {
        (self.0)(&evt.erase())
    }

    fn blocking_flush(&self, _: Duration) {}
}

pub fn from_fn<F: Fn(&Event<&dyn ErasedProps>)>(f: F) -> FromFn<F> {
    FromFn(f)
}

pub struct And<T, U> {
    left: T,
    right: U,
}

impl<T, U> And<T, U> {
    pub fn left(&self) -> &T {
        &self.left
    }

    pub fn right(&self) -> &U {
        &self.right
    }
}

impl<T: Emitter, U: Emitter> Emitter for And<T, U> {
    fn emit<P: Props>(&self, evt: &Event<P>) {
        self.left.emit(evt);
        self.right.emit(evt);
    }

    fn blocking_flush(&self, timeout: Duration) {
        // Approximate; give each target an equal
        // time to flush. With a monotonic clock
        // we could measure the time each takes
        // to flush and track in our timeout
        let timeout = timeout / 2;

        self.left.blocking_flush(timeout);
        self.right.blocking_flush(timeout);
    }
}

pub struct ByRef<'a, T: ?Sized>(&'a T);

impl<'a, T: Emitter + ?Sized> Emitter for ByRef<'a, T> {
    fn emit<P: Props>(&self, evt: &Event<P>) {
        self.0.emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) {
        self.0.blocking_flush(timeout)
    }
}

mod internal {
    use core::time::Duration;

    use crate::{event::Event, props::ErasedProps};

    pub trait DispatchEmitter {
        fn dispatch_emit(&self, evt: &Event<&dyn ErasedProps>);
        fn dispatch_blocking_flush(&self, timeout: Duration);
    }

    pub trait SealedEmitter {
        fn erase_target(&self) -> crate::internal::Erased<&dyn DispatchEmitter>;
    }
}

pub trait ErasedEmitter: internal::SealedEmitter {}

impl<T: Emitter> ErasedEmitter for T {}

impl<T: Emitter> internal::SealedEmitter for T {
    fn erase_target(&self) -> crate::internal::Erased<&dyn internal::DispatchEmitter> {
        crate::internal::Erased(self)
    }
}

impl<T: Emitter> internal::DispatchEmitter for T {
    fn dispatch_emit(&self, evt: &Event<&dyn ErasedProps>) {
        self.emit(evt)
    }

    fn dispatch_blocking_flush(&self, timeout: Duration) {
        self.blocking_flush(timeout)
    }
}

impl<'a> Emitter for dyn ErasedEmitter + 'a {
    fn emit<P: Props>(&self, evt: &Event<P>) {
        self.erase_target().0.dispatch_emit(&evt.erase())
    }

    fn blocking_flush(&self, timeout: Duration) {
        self.erase_target().0.dispatch_blocking_flush(timeout)
    }
}

impl<'a> Emitter for dyn ErasedEmitter + Send + Sync + 'a {
    fn emit<P: Props>(&self, evt: &Event<P>) {
        (self as &(dyn ErasedEmitter + 'a)).emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) {
        (self as &(dyn ErasedEmitter + 'a)).blocking_flush(timeout)
    }
}
