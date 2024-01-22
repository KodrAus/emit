use core::time::Duration;

use crate::{Clock, Extent, Timestamp, ToExtent};

#[derive(Clone, Copy)]
pub struct Timer<C> {
    start: Option<Timestamp>,
    clock: C,
}

impl<C: Clock> Timer<C> {
    pub fn start(clock: C) -> Self {
        Timer {
            start: clock.now(),
            clock,
        }
    }

    pub fn extent(&self) -> Option<Extent> {
        let end = self.clock.now();

        match (self.start, end) {
            (Some(start), Some(end)) => Extent::span(start..end),
            _ => None,
        }
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.extent().map(|extent| extent.len())
    }
}

impl<C: Clock> ToExtent for Timer<C> {
    fn to_extent(&self) -> Option<Extent> {
        self.extent()
    }
}
