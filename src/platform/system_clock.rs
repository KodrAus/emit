use emit_core::{clock::Clock, timestamp::Timestamp};

#[derive(Default, Debug, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Option<Timestamp> {
        Timestamp::new(std::time::UNIX_EPOCH.elapsed().unwrap_or_default())
    }
}
