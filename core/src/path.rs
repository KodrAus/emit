use core::fmt;

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
    pub const fn new(source: &'static str) -> Self {
        Path(Str::new(source))
    }
}

impl<'a> Path<'a> {
    pub const fn new_ref(source: &'a str) -> Self {
        Path(Str::new_ref(source))
    }

    pub fn by_ref<'b>(&'b self) -> Path<'b> {
        Path(self.0.by_ref())
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.as_str().split("::")
    }

    pub fn is_child_of<'b>(&self, other: &Path<'b>) -> bool {
        let child = self.0.as_str();
        let parent = other.0.as_str();

        if child.len() >= parent.len() && child.is_char_boundary(parent.len()) {
            let (child_prefix, child_suffix) = child.split_at(parent.len());

            child_prefix == parent && (child_suffix.is_empty() || child_suffix.starts_with("::"))
        } else {
            false
        }
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
impl<'k> sval::Value for Path<'k> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        use sval_ref::ValueRef as _;

        self.0.stream_ref(stream)
    }
}

#[cfg(feature = "sval")]
impl<'k> sval_ref::ValueRef<'k> for Path<'k> {
    fn stream_ref<S: sval::Stream<'k> + ?Sized>(&self, stream: &mut S) -> sval::Result {
        self.0.stream_ref(stream)
    }
}

#[cfg(feature = "serde")]
impl<'k> serde::Serialize for Path<'k> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
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
