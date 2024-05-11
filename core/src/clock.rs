use crate::{empty::Empty, timestamp::Timestamp};

pub trait Clock {
    fn now(&self) -> Option<Timestamp>;
}

impl<'a, T: Clock + ?Sized> Clock for &'a T {
    fn now(&self) -> Option<Timestamp> {
        (**self).now()
    }
}

impl<'a, T: Clock> Clock for Option<T> {
    fn now(&self) -> Option<Timestamp> {
        if let Some(time) = self {
            time.now()
        } else {
            Empty.now()
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Clock + ?Sized + 'a> Clock for alloc::boxed::Box<T> {
    fn now(&self) -> Option<Timestamp> {
        (**self).now()
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Clock + ?Sized + 'a> Clock for alloc::sync::Arc<T> {
    fn now(&self) -> Option<Timestamp> {
        (**self).now()
    }
}

impl Clock for Empty {
    fn now(&self) -> Option<Timestamp> {
        None
    }
}

mod internal {
    use super::Timestamp;

    pub trait DispatchClock {
        fn dispatch_now(&self) -> Option<Timestamp>;
    }

    pub trait SealedClock {
        fn erase_clock(&self) -> crate::internal::Erased<&dyn DispatchClock>;
    }
}

pub trait ErasedClock: internal::SealedClock {}

impl<T: Clock> ErasedClock for T {}

impl<T: Clock> internal::SealedClock for T {
    fn erase_clock(&self) -> crate::internal::Erased<&dyn internal::DispatchClock> {
        crate::internal::Erased(self)
    }
}

impl<T: Clock> internal::DispatchClock for T {
    fn dispatch_now(&self) -> Option<Timestamp> {
        self.now()
    }
}

impl<'a> Clock for dyn ErasedClock + 'a {
    fn now(&self) -> Option<Timestamp> {
        self.erase_clock().0.dispatch_now()
    }
}

impl<'a> Clock for dyn ErasedClock + Send + Sync + 'a {
    fn now(&self) -> Option<Timestamp> {
        (self as &(dyn ErasedClock + 'a)).now()
    }
}
