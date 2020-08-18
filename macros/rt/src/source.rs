use std::ops::Index;

use super::value::ValueBag;

#[derive(Clone, Copy)]
pub struct Source<'a> {
    pub sorted_key_values: &'a [(&'static str, ValueBag<'a>)],
}

impl<'a> Source<'a> {
    pub fn get(&self, key: impl AsRef<str>) -> Option<&ValueBag<'a>> {
        self.sorted_key_values
            .binary_search_by_key(&key.as_ref(), |(k, _)| k)
            .ok()
            .map(|index| &self.sorted_key_values[index].1)
    }
}

impl<'a, 'b> Index<&'b str> for Source<'a> {
    type Output = ValueBag<'a>;

    fn index(&self, key: &'b str) -> &ValueBag<'a> {
        self.get(key).unwrap()
    }
}

impl<'a> Index<usize> for Source<'a> {
    type Output = ValueBag<'a>;

    fn index(&self, index: usize) -> &ValueBag<'a> {
        &self.sorted_key_values[index].1
    }
}
