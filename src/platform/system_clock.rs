use emit_core::time::{Clock, Timestamp};

#[derive(Default, Debug, Clone, Copy)]
pub struct SystemClock;

impl SystemClock {
    pub fn now() -> Timestamp {
        Timestamp::new(std::time::UNIX_EPOCH.elapsed().unwrap_or_default())
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Option<Timestamp> {
        Some(Self::now())
    }
}
