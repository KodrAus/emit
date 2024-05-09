pub struct ByRef<'a, T: ?Sized>(&'a T);

impl<'a, T: ?Sized> ByRef<'a, T> {
    pub const fn new(value: &'a T) -> Self {
        ByRef(value)
    }

    pub const fn inner(&self) -> &'a T {
        self.0
    }
}
