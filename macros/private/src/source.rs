use log::kv::{Error, Key, Source, ToValue, Value, Visitor};

/**
A `kv::Source` that can find the index for a key efficiently.
*/
pub struct Captured<'a> {
    pub lookup: fn(&str) -> Option<usize>,
    pub key_values: &'a [(&'a str, Value<'a>)],
}

impl<'a> Source for Captured<'a> {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error> {
        self.key_values.visit(visitor)
    }

    fn get<'v>(&'v self, key: Key) -> Option<Value<'v>> {
        (self.lookup)(key.as_str()).map(|index| self.key_values[index].1.to_value())
    }

    fn count(&self) -> usize {
        self.key_values.len()
    }
}
