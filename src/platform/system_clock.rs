use crate::{time::Time, Timestamp};

#[derive(Default, Debug, Clone, Copy)]
pub struct SystemClock;

impl SystemClock {
    pub fn now() -> Timestamp {
        Timestamp::new(std::time::UNIX_EPOCH.elapsed().unwrap_or_default())
    }
}

impl Time for SystemClock {
    fn timestamp(&self) -> Option<Timestamp> {
        Some(Self::now())
    }
}
