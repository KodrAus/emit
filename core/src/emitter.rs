use core::time::Duration;

use crate::{
    and::And,
    empty::Empty,
    event::{Event, ToEvent},
    props::ErasedProps,
};

pub trait Emitter {
    fn emit<E: ToEvent>(&self, evt: E);

    fn blocking_flush(&self, timeout: Duration);

    fn and_to<U>(self, other: U) -> And<Self, U>
    where
        Self: Sized,
    {
        And::new(self, other)
    }
}

impl<'a, T: Emitter + ?Sized> Emitter for &'a T {
    fn emit<E: ToEvent>(&self, evt: E) {
        (**self).emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) {
        (**self).blocking_flush(timeout)
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Emitter + ?Sized + 'a> Emitter for alloc::boxed::Box<T> {
    fn emit<E: ToEvent>(&self, evt: E) {
        (**self).emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) {
        (**self).blocking_flush(timeout)
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Emitter + ?Sized + 'a> Emitter for alloc::sync::Arc<T> {
    fn emit<E: ToEvent>(&self, evt: E) {
        (**self).emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) {
        (**self).blocking_flush(timeout)
    }
}

impl<T: Emitter> Emitter for Option<T> {
    fn emit<E: ToEvent>(&self, evt: E) {
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
    fn emit<E: ToEvent>(&self, _: E) {}
    fn blocking_flush(&self, _: Duration) {}
}

impl Emitter for fn(&Event<&dyn ErasedProps>) {
    fn emit<E: ToEvent>(&self, evt: E) {
        (self)(&evt.to_event().erase())
    }

    fn blocking_flush(&self, _: Duration) {}
}

pub struct FromFn<F>(F);

impl<F: Fn(&Event<&dyn ErasedProps>)> Emitter for FromFn<F> {
    fn emit<E: ToEvent>(&self, evt: E) {
        (self.0)(&evt.to_event().erase())
    }

    fn blocking_flush(&self, _: Duration) {}
}

pub fn from_fn<F: Fn(&Event<&dyn ErasedProps>)>(f: F) -> FromFn<F> {
    FromFn(f)
}

impl<T: Emitter, U: Emitter> Emitter for And<T, U> {
    fn emit<E: ToEvent>(&self, evt: E) {
        let evt = evt.to_event();

        self.left().emit(&evt);
        self.right().emit(&evt);
    }

    fn blocking_flush(&self, timeout: Duration) {
        // Approximate; give each target an equal
        // time to flush. With a monotonic clock
        // we could measure the time each takes
        // to flush and track in our timeout
        let timeout = timeout / 2;

        self.left().blocking_flush(timeout);
        self.right().blocking_flush(timeout);
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
        fn erase_emitter(&self) -> crate::internal::Erased<&dyn DispatchEmitter>;
    }
}

pub trait ErasedEmitter: internal::SealedEmitter {}

impl<T: Emitter> ErasedEmitter for T {}

impl<T: Emitter> internal::SealedEmitter for T {
    fn erase_emitter(&self) -> crate::internal::Erased<&dyn internal::DispatchEmitter> {
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
    fn emit<E: ToEvent>(&self, evt: E) {
        self.erase_emitter()
            .0
            .dispatch_emit(&evt.to_event().erase())
    }

    fn blocking_flush(&self, timeout: Duration) {
        self.erase_emitter().0.dispatch_blocking_flush(timeout)
    }
}

impl<'a> Emitter for dyn ErasedEmitter + Send + Sync + 'a {
    fn emit<E: ToEvent>(&self, evt: E) {
        (self as &(dyn ErasedEmitter + 'a)).emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) {
        (self as &(dyn ErasedEmitter + 'a)).blocking_flush(timeout)
    }
}
