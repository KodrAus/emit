pub struct Val<'a>(value_bag::ValueBag<'a>);

impl<'a> Val<'a> {
    pub fn by_ref<'b>(&'b self) -> Val<'b> {
        Val(self.0.by_ref())
    }
}

impl<'a> From<i32> for Val<'a> {
    fn from(value: i32) -> Self {
        Val(value.into())
    }
}
