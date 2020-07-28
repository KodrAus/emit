use log::kv::{Error, Key, Source, ToValue, Value, Visitor};

/**
A `kv::Source` that can find the index for a key more efficiently than scanning.
*/
pub struct Captured<'a> {
    pub lookup: fn(&str) -> Option<usize>,
    pub key_values: &'a [(&'a str, Value<'a>)],
}

impl<'a> Captured<'a> {
    pub fn get(&self, key: impl AsRef<str>) -> Option<Value> {
        (self.lookup)(key.as_ref()).map(|index| self.key_values[index].1.to_value())
    }
}

impl<'a> Source for Captured<'a> {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error> {
        self.key_values.visit(visitor)
    }

    fn get<'v>(&'v self, key: Key) -> Option<Value<'v>> {
        self.get(key)
    }

    fn count(&self) -> usize {
        self.key_values.len()
    }
}
