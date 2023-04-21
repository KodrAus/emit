#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key<'k>(&'k str);

pub trait ToKey {
    fn to_key(&self) -> Key;
}

impl<'a, K: ToKey + ?Sized> ToKey for &'a K {
    fn to_key(&self) -> Key {
        (**self).to_key()
    }
}

impl<'k> ToKey for Key<'k> {
    fn to_key(&self) -> Key {
        Key(self.0)
    }
}
