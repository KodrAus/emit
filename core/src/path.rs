use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    str,
};

use crate::{
    str::Str,
    value::{FromValue, ToValue, Value},
};

#[derive(Clone)]
pub struct Path<'a>(Str<'a>);

impl<'a> From<&'a str> for Path<'a> {
    fn from(value: &'a str) -> Self {
        Path(Str::from(value))
    }
}

impl<'a> From<Str<'a>> for Path<'a> {
    fn from(value: Str<'a>) -> Self {
        Path(value)
    }
}

impl<'a, 'b> From<&'a Path<'b>> for Path<'a> {
    fn from(value: &'a Path<'b>) -> Self {
        value.by_ref()
    }
}

impl<'a> ToValue for Path<'a> {
    fn to_value(&self) -> Value {
        self.0.to_value()
    }
}

impl<'a> FromValue<'a> for Path<'a> {
    fn from_value(value: Value<'a>) -> Option<Self> {
        Some(value.cast()?)
    }
}

impl Path<'static> {
    pub const fn new(path: &'static str) -> Self {
        Path::new_str(Str::new(path))
    }
}

impl<'a> Path<'a> {
    pub const fn new_ref(path: &'a str) -> Self {
        Self::new_str(Str::new_ref(path))
    }

    pub const fn new_str(path: Str<'a>) -> Self {
        Path(path)
    }

    pub fn by_ref<'b>(&'b self) -> Path<'b> {
        Path(self.0.by_ref())
    }

    pub fn segments(&self) -> Segments {
        Segments {
            inner: self.0.get().split("::"),
        }
    }

    pub fn as_str(&self) -> Option<&Str<'a>> {
        Some(&self.0)
    }

    pub fn is_child_of<'b>(&self, other: &Path<'b>) -> bool {
        let child = self.0.get();
        let parent = other.0.get();

        if child.len() >= parent.len() && child.is_char_boundary(parent.len()) {
            let (child_prefix, child_suffix) = child.split_at(parent.len());

            child_prefix == parent && (child_suffix.is_empty() || child_suffix.starts_with("::"))
        } else {
            false
        }
    }
}

pub struct Segments<'a> {
    inner: str::Split<'a, &'static str>,
}

impl<'a> Iterator for Segments<'a> {
    type Item = Str<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Str::new_ref)
    }
}

impl<'a> Eq for Path<'a> {}

impl<'a, 'b> PartialEq<Path<'b>> for Path<'a> {
    fn eq(&self, other: &Path<'b>) -> bool {
        self.0 == other.0
    }
}

impl<'a> Hash for Path<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<'a, 'b> PartialEq<Str<'b>> for Path<'a> {
    fn eq(&self, other: &Str<'b>) -> bool {
        self.0 == *other
    }
}

impl<'a, 'b> PartialEq<Path<'b>> for Str<'a> {
    fn eq(&self, other: &Path<'b>) -> bool {
        *self == other.0
    }
}

impl<'a> PartialEq<str> for Path<'a> {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl<'a> PartialEq<Path<'a>> for str {
    fn eq(&self, other: &Path<'a>) -> bool {
        self == other.0
    }
}

impl<'a, 'b> PartialEq<&'b str> for Path<'a> {
    fn eq(&self, other: &&'b str) -> bool {
        self.0 == *other
    }
}

impl<'a> PartialOrd for Path<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<'a> Ord for Path<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<'a> fmt::Debug for Path<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl<'a> fmt::Display for Path<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(feature = "sval")]
impl<'a> sval::Value for Path<'a> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        use sval_ref::ValueRef as _;

        self.0.stream_ref(stream)
    }
}

#[cfg(feature = "sval")]
impl<'a> sval_ref::ValueRef<'a> for Path<'a> {
    fn stream_ref<S: sval::Stream<'a> + ?Sized>(&self, stream: &mut S) -> sval::Result {
        self.0.stream_ref(stream)
    }
}

#[cfg(feature = "serde")]
impl<'a> serde::Serialize for Path<'a> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use alloc::borrow::Cow;

    use super::*;

    impl Path<'static> {
        pub fn new_owned(path: impl Into<Box<str>>) -> Self {
            Path(Str::new_owned(path))
        }
    }

    impl<'a> Path<'a> {
        pub fn new_cow_ref(path: Cow<'a, str>) -> Self {
            Path(Str::new_cow_ref(path))
        }

        pub fn to_cow(&self) -> Cow<'a, str> {
            self.0.to_cow()
        }

        pub fn to_owned(&self) -> Path<'static> {
            Path(self.0.to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_child_of() {
        let a = Path::new("a");
        let aa = Path::new("aa");
        let b = Path::new("b");
        let a_b = Path::new("a::b");

        assert!(!aa.is_child_of(&a));
        assert!(!b.is_child_of(&a));
        assert!(!a.is_child_of(&a_b));

        assert!(a.is_child_of(&a));
        assert!(a_b.is_child_of(&a));
    }
}
