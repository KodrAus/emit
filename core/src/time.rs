use core::{fmt, time::Duration};

use crate::empty::Empty;

#[derive(Clone, Copy)]
pub struct Timestamp(Duration);

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl Timestamp {
    pub fn new(elapsed_since_unix_epoch: Duration) -> Self {
        Timestamp(elapsed_since_unix_epoch)
    }

    pub fn to_unix(&self) -> Duration {
        self.0
    }

    #[cfg(feature = "std")]
    pub fn to_system_time(&self) -> std::time::SystemTime {
        std::time::SystemTime::UNIX_EPOCH + self.0
    }
}

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

    pub trait SealedTime {
        fn erase_clock(&self) -> crate::internal::Erased<&dyn DispatchClock>;
    }
}

pub trait ErasedClock: internal::SealedTime {}

impl<T: Clock> ErasedClock for T {}

impl<T: Clock> internal::SealedTime for T {
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
