pub struct Chain<T, U> {
    pub(crate) first: T,
    pub(crate) second: U,
}

pub struct ByRef<'a, T: ?Sized>(pub(crate) &'a T);

pub struct Empty;

impl Empty {
    pub fn chain<U>(self, other: U) -> Chain<Self, U> {
        Chain {
            first: self,
            second: other,
        }
    }

    pub fn by_ref<'a>(&'a self) -> ByRef<'a, Self> {
        ByRef(self)
    }
}
