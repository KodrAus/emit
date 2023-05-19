use core::{fmt, time::Duration};

pub use crate::empty::Empty;

pub trait Time {
    fn timestamp(&self) -> Option<Timestamp>;
}

impl<'a, T: Time + ?Sized> Time for &'a T {
    fn timestamp(&self) -> Option<Timestamp> {
        (**self).timestamp()
    }
}

impl<'a, T: Time> Time for Option<T> {
    fn timestamp(&self) -> Option<Timestamp> {
        if let Some(time) = self {
            time.timestamp()
        } else {
            Empty.timestamp()
        }
    }
}

#[cfg(feature = "std")]
impl<'a, T: Time + ?Sized + 'a> Time for Box<T> {
    fn timestamp(&self) -> Option<Timestamp> {
        (**self).timestamp()
    }
}

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

    #[cfg(feature = "std")]
    pub fn now() -> Self {
        crate::ambient::get()
            .timestamp()
            .unwrap_or_else(SystemClock::now)
    }
}

impl Time for Timestamp {
    fn timestamp(&self) -> Option<Timestamp> {
        Some(*self)
    }
}

impl Time for Empty {
    fn timestamp(&self) -> Option<Timestamp> {
        None
    }
}

mod internal {
    use super::Timestamp;

    pub trait DispatchTime {
        fn dispatch_timestamp(&self) -> Option<Timestamp>;
    }

    pub trait SealedTime {
        fn erase_time(&self) -> crate::internal::Erased<&dyn DispatchTime>;
    }
}

pub trait ErasedTime: internal::SealedTime {}

impl<T: Time> ErasedTime for T {}

impl<T: Time> internal::SealedTime for T {
    fn erase_time(&self) -> crate::internal::Erased<&dyn internal::DispatchTime> {
        crate::internal::Erased(self)
    }
}

impl<T: Time> internal::DispatchTime for T {
    fn dispatch_timestamp(&self) -> Option<Timestamp> {
        self.timestamp()
    }
}

impl<'a> Time for dyn ErasedTime + 'a {
    fn timestamp(&self) -> Option<Timestamp> {
        self.erase_time().0.dispatch_timestamp()
    }
}

impl<'a> Time for dyn ErasedTime + Send + Sync + 'a {
    fn timestamp(&self) -> Option<Timestamp> {
        (self as &(dyn ErasedTime + 'a)).timestamp()
    }
}

#[cfg(feature = "std")]
pub(crate) struct SystemClock;

#[cfg(feature = "std")]
impl SystemClock {
    fn now() -> Timestamp {
        Timestamp::new(std::time::UNIX_EPOCH.elapsed().unwrap_or_default())
    }
}

#[cfg(feature = "std")]
impl Time for SystemClock {
    fn timestamp(&self) -> Option<Timestamp> {
        Some(Self::now())
    }
}
