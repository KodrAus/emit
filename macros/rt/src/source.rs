use super::value::ValueBag;

#[derive(Clone, Copy)]
pub struct Source<'a> {
    pub sorted_key_values: &'a [(&'a str, ValueBag<'a>)],
}

impl<'a> Source<'a> {
    pub fn get(&self, key: impl AsRef<str>) -> Option<ValueBag> {
        self.sorted_key_values
            .binary_search_by_key(&key.as_ref(), |(k, _)| k)
            .ok()
            .map(|index| self.sorted_key_values[index].1.clone())
    }
}
