pub struct Val<'a>(value_bag::ValueBag<'a>);

pub trait ToVal {
    fn to_val(&self) -> Val;
}

impl<'a, V: ToVal + ?Sized> ToVal for &'a V {
    fn to_val(&self) -> Val {
        (**self).to_val()
    }
}

impl<'v> ToVal for Val<'v> {
    fn to_val(&self) -> Val {
        Val(self.0.by_ref())
    }
}

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
