pub use value_bag::ValueBag;

pub struct Record<'a>(pub(crate) &'a crate::rt::__private::Record<'a>);

impl<'a> Record<'a> {
    pub fn get(&self, key: impl AsRef<str>) -> Option<ValueBag> {
        self.0.get(key).map(|value| value.by_ref())
    }
}
