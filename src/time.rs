use core::{fmt, time::Duration};

pub use crate::adapt::Empty;

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

#[cfg(feature = "std")]
pub(crate) struct SystemClock;

#[cfg(feature = "std")]
impl Time for SystemClock {
    fn timestamp(&self) -> Option<Timestamp> {
        Some(Timestamp::new(
            std::time::UNIX_EPOCH.elapsed().unwrap_or_default(),
        ))
    }
}
