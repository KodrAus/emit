use core::fmt;

pub struct Value<'a>(value_bag::ValueBag<'a>);

impl<'a> Value<'a> {
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

impl<'a> From<i32> for Value<'a> {
    fn from(value: i32) -> Self {
        Value(value.into())
    }
}
