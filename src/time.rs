use core::{fmt, time::Duration};

pub trait Time {
    fn get(&self) -> Timestamp;
}

impl<'a, T: Time + ?Sized> Time for &'a T {
    fn get(&self) -> Timestamp {
        (**self).get()
    }
}

#[cfg(feature = "std")]
impl<'a, T: Time + ?Sized + 'a> Time for Box<T> {
    fn get(&self) -> Timestamp {
        (**self).get()
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
        crate::TIME
            .get()
            .map(|time| time.get())
            .unwrap_or_else(|| SystemClock.get())
    }
}

impl Time for Timestamp {
    fn get(&self) -> Timestamp {
        *self
    }
}

pub(crate) enum Unsupported {}

impl Time for Unsupported {
    fn get(&self) -> Timestamp {
        unreachable!()
    }
}

#[cfg(feature = "std")]
pub(crate) struct SystemClock;

#[cfg(feature = "std")]
impl Time for SystemClock {
    fn get(&self) -> Timestamp {
        Timestamp::new(std::time::UNIX_EPOCH.elapsed().unwrap_or_default())
    }
}
