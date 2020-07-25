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

#[cfg(test)]
mod tests {
    use super::*;

    #[bench]
    fn lookup_hit_1(b: &mut test::Bencher) {
        let source = Captured {
            lookup: |key| match key {
                "a" => Some(0),
                _ => None,
            },
            key_values: &[("a", 42.into())],
        };

        b.iter(|| test::black_box(source.get(Key::from_str("a"))))
    }

    #[bench]
    fn lookup_miss_1(b: &mut test::Bencher) {
        let source = Captured {
            lookup: |key| match key {
                "a" => Some(0),
                _ => None,
            },
            key_values: &[("a", 42.into())],
        };

        b.iter(|| test::black_box(source.get(Key::from_str("b"))))
    }

    #[bench]
    fn lookup_hit_10(b: &mut test::Bencher) {
        let source = Captured {
            lookup: |key| match key {
                "a" => Some(0),
                "b" => Some(1),
                "c" => Some(2),
                "d" => Some(3),
                "e" => Some(4),
                "f" => Some(5),
                "g" => Some(6),
                "h" => Some(7),
                "i" => Some(8),
                "j" => Some(9),
                _ => None,
            },
            key_values: &[
                ("a", 1.into()),
                ("b", 1.into()),
                ("c", 1.into()),
                ("d", 1.into()),
                ("e", 1.into()),
                ("f", 1.into()),
                ("g", 1.into()),
                ("h", 1.into()),
                ("i", 1.into()),
                ("j", 1.into()),
            ],
        };

        b.iter(|| test::black_box(source.get(Key::from_str("a"))))
    }

    #[bench]
    fn lookup_miss_10(b: &mut test::Bencher) {
        let source = Captured {
            lookup: |key| match key {
                "a" => Some(0),
                "b" => Some(1),
                "c" => Some(2),
                "d" => Some(3),
                "e" => Some(4),
                "f" => Some(5),
                "g" => Some(6),
                "h" => Some(7),
                "i" => Some(8),
                "j" => Some(9),
                _ => None,
            },
            key_values: &[
                ("a", 1.into()),
                ("b", 1.into()),
                ("c", 1.into()),
                ("d", 1.into()),
                ("e", 1.into()),
                ("f", 1.into()),
                ("g", 1.into()),
                ("h", 1.into()),
                ("i", 1.into()),
                ("j", 1.into()),
            ],
        };

        b.iter(|| test::black_box(source.get(Key::from_str("k"))))
    }
}
