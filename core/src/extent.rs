use crate::{
    empty::Empty,
    key::{Key, ToKey},
    props::{ControlFlow, Props},
    time::{Clock, Timer, Timestamp},
    value::{ToValue, Value},
    well_known::{TIMESTAMP_KEY, TIMESTAMP_START_KEY},
};
use core::ops::Range;

#[derive(Debug, Clone)]
pub struct Extent(Option<Range<Timestamp>>);

impl Extent {
    pub fn point(ts: Timestamp) -> Self {
        Extent(Some(ts..ts))
    }

    pub fn span(ts: Range<Timestamp>) -> Self {
        Extent(Some(ts))
    }

    pub fn empty() -> Self {
        Extent(None)
    }

    pub fn to_point(&self) -> Option<&Timestamp> {
        self.0.as_ref().map(|ts| &ts.end)
    }

    pub fn to_span(&self) -> Option<&Range<Timestamp>> {
        self.0.as_ref().filter(|ts| ts.start != ts.end)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }
}

impl Props for Extent {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow {
        if let Some(ref ts) = self.0 {
            if ts.start != ts.end {
                for_each(TIMESTAMP_START_KEY.to_key(), ts.start.to_value())?;
            }

            for_each(TIMESTAMP_KEY.to_key(), ts.end.to_value())
        } else {
            ControlFlow::Continue(())
        }
    }
}

pub trait ToExtent {
    fn to_extent(&self) -> Extent;
}

impl<'a, T: ToExtent + ?Sized> ToExtent for &'a T {
    fn to_extent(&self) -> Extent {
        (**self).to_extent()
    }
}

impl ToExtent for Empty {
    fn to_extent(&self) -> Extent {
        Extent::empty()
    }
}

impl<T: ToExtent> ToExtent for Option<T> {
    fn to_extent(&self) -> Extent {
        self.as_ref()
            .map(|ts| ts.to_extent())
            .unwrap_or_else(Extent::empty)
    }
}

impl ToExtent for Timestamp {
    fn to_extent(&self) -> Extent {
        Extent::point(*self)
    }
}

impl ToExtent for Range<Timestamp> {
    fn to_extent(&self) -> Extent {
        Extent::span(self.clone())
    }
}

impl<C: Clock> ToExtent for Timer<C> {
    fn to_extent(&self) -> Extent {
        self.extent()
    }
}
