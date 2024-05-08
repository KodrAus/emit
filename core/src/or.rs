pub struct Or<T, U> {
    left: T,
    right: U,
}

impl<T, U> Or<T, U> {
    pub const fn new(left: T, right: U) -> Self {
        Or { left, right }
    }

    pub fn left(&self) -> &T {
        &self.left
    }

    pub fn left_mut(&mut self) -> &mut T {
        &mut self.left
    }

    pub fn right(&self) -> &U {
        &self.right
    }

    pub fn right_mut(&mut self) -> &mut U {
        &mut self.right
    }

    pub fn into_inner(self) -> (T, U) {
        (self.left, self.right)
    }
}
