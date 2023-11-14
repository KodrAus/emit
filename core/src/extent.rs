use crate::{empty::Empty, timestamp::Timestamp};
use core::{fmt, ops::Range, time::Duration};

#[derive(Debug, Clone)]
pub struct Extent {
    range: Range<Timestamp>,
    is_span: bool,
}

impl Extent {
    pub fn point(ts: Timestamp) -> Self {
        Extent {
            range: ts..ts,
            is_span: false,
        }
    }

    pub fn span(ts: Range<Timestamp>) -> Option<Self> {
        if ts.start > ts.end {
            None
        } else {
            Some(Extent {
                range: ts,
                is_span: true,
            })
        }
    }

    pub fn as_range(&self) -> &Range<Timestamp> {
        &self.range
    }

    pub fn as_point(&self) -> Option<&Timestamp> {
        if self.is_point() {
            Some(&self.range.end)
        } else {
            None
        }
    }

    pub fn to_point(&self) -> &Timestamp {
        &self.range.end
    }

    pub fn as_span(&self) -> Option<&Range<Timestamp>> {
        if self.is_span() {
            Some(&self.range)
        } else {
            None
        }
    }

    pub fn len(&self) -> Duration {
        self.range
            .end
            .duration_since(self.range.start)
            .expect("end is always after start")
    }

    pub fn is_point(&self) -> bool {
        !self.is_span()
    }

    pub fn is_span(&self) -> bool {
        self.is_span
    }
}

impl fmt::Display for Extent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_span() {
            fmt::Display::fmt(&self.range.start, f)?;
            f.write_str("..")?;
            fmt::Display::fmt(&self.range.end, f)
        } else {
            fmt::Display::fmt(&self.range.end, f)
        }
    }
}

pub trait ToExtent {
    fn to_extent(&self) -> Option<Extent>;
}

impl<'a, T: ToExtent + ?Sized> ToExtent for &'a T {
    fn to_extent(&self) -> Option<Extent> {
        (**self).to_extent()
    }
}

impl ToExtent for Empty {
    fn to_extent(&self) -> Option<Extent> {
        None
    }
}

impl<T: ToExtent> ToExtent for Option<T> {
    fn to_extent(&self) -> Option<Extent> {
        self.as_ref().and_then(|ts| ts.to_extent())
    }
}

impl ToExtent for Extent {
    fn to_extent(&self) -> Option<Extent> {
        Some(self.clone())
    }
}

impl ToExtent for Timestamp {
    fn to_extent(&self) -> Option<Extent> {
        Some(Extent::point(*self))
    }
}

impl ToExtent for Range<Timestamp> {
    fn to_extent(&self) -> Option<Extent> {
        Extent::span(self.clone())
    }
}

impl ToExtent for Range<Option<Timestamp>> {
    fn to_extent(&self) -> Option<Extent> {
        match (self.start, self.end) {
            (Some(start), Some(end)) => (start..end).to_extent(),
            (Some(start), None) => start.to_extent(),
            (None, Some(end)) => end.to_extent(),
            (None, None) => None::<Timestamp>.to_extent(),
        }
    }
}
