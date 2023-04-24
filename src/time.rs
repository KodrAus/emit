use core::time::Duration;

pub trait Time {
    fn get(&self) -> Timestamp;
}

impl<'a, T: Time + ?Sized> Time for &'a T {
    fn get(&self) -> Timestamp {
        (**self).get()
    }
}

#[derive(Clone, Copy)]
pub struct Timestamp(Duration);

impl Timestamp {
    pub fn new(elapsed_since_unix_epoch: Duration) -> Self {
        Timestamp(elapsed_since_unix_epoch)
    }
}

impl Time for Timestamp {
    fn get(&self) -> Timestamp {
        *self
    }
}
