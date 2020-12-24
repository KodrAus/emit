use crate::std::{fmt, ops::Index};

use super::value::ValueBag;

#[derive(Clone, Copy)]
pub struct KeyValues<'a> {
    pub sorted_key_values: &'a [(&'static str, ValueBag<'a>)],
}

impl<'a> KeyValues<'a> {
    pub fn get(&self, key: impl AsRef<str>) -> Option<&ValueBag<'a>> {
        self.sorted_key_values
            .binary_search_by_key(&key.as_ref(), |(k, _)| k)
            .ok()
            .map(|index| &self.sorted_key_values[index].1)
    }
}

impl<'a, 'b> Index<&'b str> for KeyValues<'a> {
    type Output = ValueBag<'a>;

    fn index(&self, key: &'b str) -> &ValueBag<'a> {
        self.get(key).unwrap()
    }
}

impl<'a> Index<usize> for KeyValues<'a> {
    type Output = ValueBag<'a>;

    fn index(&self, index: usize) -> &ValueBag<'a> {
        &self.sorted_key_values[index].1
    }
}

impl<'a> fmt::Debug for KeyValues<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.sorted_key_values, f)
    }
}
