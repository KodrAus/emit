use core::fmt;

pub struct Value<'a>(value_bag::ValueBag<'a>);

impl<'a> Value<'a> {
    pub fn to_borrowed_str(&self) -> Option<&str> {
        self.0.to_borrowed_str()
    }

    pub fn by_ref<'b>(&'b self) -> Value<'b> {
        Value(self.0.by_ref())
    }
}

impl<'a> fmt::Debug for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl<'a> fmt::Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<'a> From<value_bag::ValueBag<'a>> for Value<'a> {
    fn from(value: value_bag::ValueBag<'a>) -> Self {
        Value(value)
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(value: &'a str) -> Self {
        Value(value.into())
    }
}

impl<'a> From<i32> for Value<'a> {
    fn from(value: i32) -> Self {
        Value(value.into())
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    #[derive(Clone)]
    pub struct OwnedValue(value_bag::OwnedValueBag);

    impl<'v> Value<'v> {
        pub fn to_owned(&self) -> OwnedValue {
            OwnedValue(self.0.to_owned())
        }
    }

    impl<'v> From<&'v OwnedValue> for Value<'v> {
        fn from(value: &'v OwnedValue) -> Value<'v> {
            value.by_ref()
        }
    }

    impl OwnedValue {
        pub fn by_ref<'v>(&'v self) -> Value<'v> {
            Value(self.0.by_ref())
        }
    }
}

#[cfg(feature = "alloc")]
pub use self::alloc_support::*;
