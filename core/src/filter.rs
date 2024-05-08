use core::time::Duration;

use crate::{
    and::And,
    emitter::Emitter,
    empty::Empty,
    event::{Event, ToEvent},
    or::Or,
    props::ErasedProps,
};

pub trait Filter {
    fn matches<E: ToEvent>(&self, evt: E) -> bool;

    fn and_when<U>(self, other: U) -> And<Self, U>
    where
        Self: Sized,
    {
        And::new(self, other)
    }

    fn or_when<U>(self, other: U) -> Or<Self, U>
    where
        Self: Sized,
    {
        Or::new(self, other)
    }

    fn wrap_emitter<E>(self, emitter: E) -> Wrap<Self, E>
    where
        Self: Sized,
    {
        Wrap {
            filter: self,
            emitter,
        }
    }

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }
}

impl<'a, F: Filter + ?Sized> Filter for &'a F {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        (**self).matches(evt)
    }
}

#[cfg(feature = "alloc")]
impl<'a, F: Filter + ?Sized + 'a> Filter for alloc::boxed::Box<F> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        (**self).matches(evt)
    }
}

impl<F: Filter> Filter for Option<F> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        match self {
            Some(filter) => filter.matches(evt),
            None => Empty.matches(evt),
        }
    }
}

impl Filter for Empty {
    fn matches<E: ToEvent>(&self, _: E) -> bool {
        true
    }
}

impl Filter for fn(&Event<&dyn ErasedProps>) -> bool {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        (self)(&evt.to_event().erase())
    }
}

pub struct FromFn<F>(F);

impl<F: Fn(&Event<&dyn ErasedProps>) -> bool> Filter for FromFn<F> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        (self.0)(&evt.to_event().erase())
    }
}

pub fn from_fn<F: Fn(&Event<&dyn ErasedProps>) -> bool>(f: F) -> FromFn<F> {
    FromFn(f)
}

pub struct Wrap<F, E> {
    filter: F,
    emitter: E,
}

impl<F: Filter, E: Emitter> Emitter for Wrap<F, E> {
    fn emit<T: ToEvent>(&self, evt: T) {
        let evt = evt.to_event();

        if self.filter.matches(&evt) {
            self.emitter.emit(evt);
        }
    }

    fn blocking_flush(&self, timeout: Duration) {
        self.emitter.blocking_flush(timeout)
    }
}

pub fn wrap<F: Filter, E: Emitter>(filter: F, emitter: E) -> Wrap<F, E> {
    filter.wrap_emitter(emitter)
}

impl<T: Filter, U: Filter> Filter for And<T, U> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        let evt = evt.to_event();

        self.left().matches(&evt) && self.right().matches(&evt)
    }
}

impl<T: Filter, U: Filter> Filter for Or<T, U> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        let evt = evt.to_event();

        self.left().matches(&evt) || self.right().matches(&evt)
    }
}

pub struct ByRef<'a, T: ?Sized>(&'a T);

impl<'a, T: Filter + ?Sized> Filter for ByRef<'a, T> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        self.0.matches(evt)
    }
}

pub fn always() -> Empty {
    Empty
}

mod internal {
    use crate::{event::Event, props::ErasedProps};

    pub trait DispatchFilter {
        fn dispatch_matches(&self, evt: &Event<&dyn ErasedProps>) -> bool;
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
    fn dispatch_matches(&self, evt: &Event<&dyn ErasedProps>) -> bool {
        self.matches(evt)
    }
}

impl<'a> Filter for dyn ErasedFilter + 'a {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        self.erase_when()
            .0
            .dispatch_matches(&evt.to_event().erase())
    }
}

impl<'a> Filter for dyn ErasedFilter + Send + Sync + 'a {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        (self as &(dyn ErasedFilter + 'a)).matches(evt)
    }
}
