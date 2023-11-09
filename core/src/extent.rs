use crate::{empty::Empty, timestamp::Timestamp};
use core::{fmt, ops::Range, time::Duration};

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

    pub fn as_range(&self) -> Option<&Range<Timestamp>> {
        self.0.as_ref()
    }

    pub fn as_point(&self) -> Option<&Timestamp> {
        if self.is_point() {
            self.to_point()
        } else {
            None
        }
    }

    pub fn to_point(&self) -> Option<&Timestamp> {
        Some(&self.0.as_ref()?.end)
    }

    pub fn as_span(&self) -> Option<&Range<Timestamp>> {
        let ts = self.0.as_ref()?;

        if ts.start != ts.end {
            Some(ts)
        } else {
            None
        }
    }

    pub fn len(&self) -> Option<Duration> {
        let ts = self.0.as_ref()?;

        ts.end.duration_since(ts.start)
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

    pub fn or_else<T: ToExtent>(&self, or_else: impl FnOnce() -> T) -> Self {
        if self.0.is_none() {
            or_else().to_extent()
        } else {
            self.clone()
        }
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

impl ToExtent for Range<Option<Timestamp>> {
    fn to_extent(&self) -> Extent {
        match (self.start, self.end) {
            (Some(start), Some(end)) => (start..end).to_extent(),
            (Some(start), None) => start.to_extent(),
            (None, Some(end)) => end.to_extent(),
            (None, None) => None::<Timestamp>.to_extent(),
        }
    }
}
