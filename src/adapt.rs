pub struct Chain<T, U> {
    pub(crate) first: T,
    pub(crate) second: U,
}

pub struct ByRef<'a, T: ?Sized>(pub(crate) &'a T);
