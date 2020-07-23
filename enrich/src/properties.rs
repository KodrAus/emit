use std::{
    collections::hash_map::{self, HashMap},
    mem,
};

use stdlog::kv::{self, Source};
use sval::value::{self, OwnedValue};

/**
A map of enriched properties.

This map is optimised for contexts that are empty or contain a single property.
*/
#[derive(Clone, Debug)]
pub(crate) enum Properties {
    Empty,
    Single(&'static str, OwnedValue),
    Map(HashMap<&'static str, OwnedValue>),
}

pub(crate) enum PropertiesIter<'a> {
    Empty,
    Single(&'static str, &'a OwnedValue),
    Map(hash_map::Iter<'a, &'static str, OwnedValue>),
}

impl<'a> Iterator for PropertiesIter<'a> {
    type Item = (&'static str, &'a OwnedValue);

    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            PropertiesIter::Empty => None,
            PropertiesIter::Single(k, v) => {
                *self = PropertiesIter::Empty;

                Some((k, v))
            }
            PropertiesIter::Map(ref mut map) => map.next().map(|(k, v)| (*k, v)),
        }
    }
}

impl Default for Properties {
    fn default() -> Self {
        Properties::Empty
    }
}

impl Properties {
    pub fn insert(&mut self, k: &'static str, v: OwnedValue) {
        match *self {
            Properties::Empty => {
                *self = Properties::Single(k, v);
            }
            Properties::Single(_, _) => {
                if let Properties::Single(pk, pv) =
                    mem::replace(self, Properties::Map(HashMap::new()))
                {
                    self.insert(pk, pv);
                    self.insert(k, v);
                } else {
                    unreachable!()
                }
            }
            Properties::Map(ref mut m) => {
                m.insert(k, v);
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&OwnedValue> {
        match self {
            Properties::Single(k, v) if *k == key => Some(v),
            Properties::Map(m) => m.get(key),
            _ => None,
        }
    }

    pub fn contains_key(&self, key: &str) -> bool {
        match self {
            Properties::Single(k, _) if *k == key => true,
            Properties::Map(m) => m.contains_key(key),
            _ => false,
        }
    }

    pub fn iter(&self) -> PropertiesIter {
        self.into_iter()
    }

    pub fn len(&self) -> usize {
        match *self {
            Properties::Empty => 0,
            Properties::Single(_, _) => 1,
            Properties::Map(ref m) => m.len(),
        }
    }
}

impl<'a> Extend<(&'static str, &'a OwnedValue)> for Properties {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = (&'static str, &'a OwnedValue)>,
    {
        for (k, v) in iter {
            if !self.contains_key(k) {
                self.insert(k, v.to_owned());
            }
        }
    }
}

impl<'a> IntoIterator for &'a Properties {
    type Item = (&'static str, &'a OwnedValue);
    type IntoIter = PropertiesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        match *self {
            Properties::Empty => PropertiesIter::Empty,
            Properties::Single(ref k, ref v) => PropertiesIter::Single(k, v),
            Properties::Map(ref m) => PropertiesIter::Map(m.iter()),
        }
    }
}

impl value::Value for Properties {
    fn stream(&self, stream: &mut value::Stream) -> value::Result {
        stream.map_begin(Some(self.len()))?;

        for (k, v) in self {
            stream.map_key(k)?;
            stream.map_value(v)?;
        }

        stream.map_end()
    }
}

impl Source for Properties {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn kv::Visitor<'kvs>) -> Result<(), kv::Error> {
        for (k, v) in self {
            visitor.visit_pair(kv::Key::from_str(k), kv::Value::from_sval(v))?;
        }

        Ok(())
    }

    fn get<'v>(&'v self, key: kv::Key) -> Option<kv::Value<'v>> {
        self.get(key.as_str()).map(|v| kv::Value::from_sval(v))
    }

    fn count(&self) -> usize {
        self.len()
    }
}
