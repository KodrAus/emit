use crate::{
    empty::Empty,
    event::Event,
    time::{Clock, Timer, Timestamp},
};
use core::ops::Range;

pub trait Extent {
    fn extent(&self) -> Option<Range<Timestamp>>;
}

impl<'a, T: Extent + ?Sized> Extent for &'a T {
    fn extent(&self) -> Option<Range<Timestamp>> {
        (**self).extent()
    }
}

impl Extent for Empty {
    fn extent(&self) -> Option<Range<Timestamp>> {
        None
    }
}

impl<'a, P> Extent for Event<'a, P> {
    fn extent(&self) -> Option<Range<Timestamp>> {
        self.extent().cloned()
    }
}

impl Extent for Timestamp {
    fn extent(&self) -> Option<Range<Timestamp>> {
        Some(*self..*self)
    }
}

impl Extent for Range<Timestamp> {
    fn extent(&self) -> Option<Range<Timestamp>> {
        Some(self.clone())
    }
}

impl<C: Clock> Extent for Timer<C> {
    fn extent(&self) -> Option<Range<Timestamp>> {
        self.extent()
    }
}

impl<T: Extent> Extent for Option<T> {
    fn extent(&self) -> Option<Range<Timestamp>> {
        self.as_ref().and_then(|ts| ts.extent())
    }
}
