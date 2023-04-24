pub struct Value<'a>(value_bag::ValueBag<'a>);

impl<'a> Value<'a> {
    pub fn by_ref<'b>(&'b self) -> Value<'b> {
        Value(self.0.by_ref())
    }
}

impl<'a> From<i32> for Value<'a> {
    fn from(value: i32) -> Self {
        Value(value.into())
    }
}
