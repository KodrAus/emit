use crate::{
    empty::Empty,
    key::{Key, ToKey},
    props::{ControlFlow, Props},
    time::{Clock, Timer, Timestamp},
    value::{ToValue, Value},
    well_known::{TIMESTAMP_KEY, TIMESTAMP_START_KEY},
};
use core::{fmt, ops::Range};

#[derive(Debug, Clone)]
pub struct Extent(Option<Range<Timestamp>>);

impl Extent {
    pub fn new(ts: Range<Timestamp>) -> Self {
        Extent(Some(ts))
    }

    pub fn point(ts: Timestamp) -> Self {
        Extent(Some(ts..ts))
    }

    pub fn empty() -> Self {
        Extent(None)
    }

    pub fn range(&self) -> Option<&Range<Timestamp>> {
        self.0.as_ref()
    }

    pub fn to_point(&self) -> Option<&Timestamp> {
        self.0.as_ref().map(|ts| &ts.end)
    }

    pub fn is_point(&self) -> bool {
        self.0
            .as_ref()
            .map(|ts| ts.start == ts.end)
            .unwrap_or(false)
    }

    pub fn is_span(&self) -> bool {
        self.0
            .as_ref()
            .map(|ts| ts.start != ts.end)
            .unwrap_or(false)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }
}

impl From<Timestamp> for Extent {
    fn from(value: Timestamp) -> Self {
        Extent::point(value)
    }
}

impl From<Range<Timestamp>> for Extent {
    fn from(value: Range<Timestamp>) -> Self {
        Extent::new(value)
    }
}

impl From<Option<Range<Timestamp>>> for Extent {
    fn from(value: Option<Range<Timestamp>>) -> Self {
        Extent(value)
    }
}

impl fmt::Display for Extent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Some(ref ts) if ts.start != ts.end => {
                fmt::Display::fmt(&ts.start, f)?;
                f.write_str("..")?;
                fmt::Display::fmt(&ts.end, f)
            }
            Some(ref ts) => fmt::Display::fmt(&ts.end, f),
            None => Ok(()),
        }
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

impl ToExtent for Extent {
    fn to_extent(&self) -> Extent {
        self.clone()
    }
}

impl ToExtent for Timestamp {
    fn to_extent(&self) -> Extent {
        Extent::point(*self)
    }
}

impl ToExtent for Range<Timestamp> {
    fn to_extent(&self) -> Extent {
        Extent::new(self.clone())
    }
}

impl<C: Clock> ToExtent for Timer<C> {
    fn to_extent(&self) -> Extent {
        self.extent()
    }
}
