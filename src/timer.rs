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
            (Some(start), Some(end)) => Some(Extent::span(start..end)),
            _ => None,
        }
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.extent().and_then(|extent| extent.len())
    }

    pub fn on_drop<F: FnOnce(Option<Extent>)>(self, complete: F) -> TimerGuard<C, F> {
        TimerGuard::new(self, complete)
    }
}

impl<C: Clock> ToExtent for Timer<C> {
    fn to_extent(&self) -> Option<Extent> {
        self.extent()
    }
}

pub struct TimerGuard<C: Clock, F: FnOnce(Option<Extent>)> {
    timer: Option<Timer<C>>,
    on_drop: Option<F>,
}

impl<C: Clock, F: FnOnce(Option<Extent>)> Drop for TimerGuard<C, F> {
    fn drop(&mut self) {
        if let Some(on_drop) = self.on_drop.take() {
            if let Some(ref timer) = self.timer {
                (on_drop)(timer.extent());
            }
        }
    }
}

impl<C: Clock, F: FnOnce(Option<Extent>)> TimerGuard<C, F> {
    pub fn new(timer: Timer<C>, on_drop: F) -> Self {
        TimerGuard {
            timer: Some(timer),
            on_drop: Some(on_drop),
        }
    }

    pub fn disabled() -> Self {
        TimerGuard {
            timer: None,
            on_drop: None,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.timer.is_some()
    }

    pub fn timer(&self) -> Option<&Timer<C>> {
        self.timer.as_ref()
    }

    pub fn complete(mut self, complete: impl FnOnce(Option<Extent>)) -> bool {
        let _ = self.on_drop.take();

        if let Some(ref timer) = self.timer {
            complete(timer.extent());
            true
        } else {
            false
        }
    }
}
