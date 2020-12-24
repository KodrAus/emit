use crate::std::{
    fmt,
    ops::Index,
};
use sval::value::{self, Value};

#[cfg(feature = "serde")]
use serde_lib::ser::{Serialize, Serializer, SerializeMap};

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

    pub fn render_debug<'b>(&'b self) -> impl fmt::Display + 'b {
        struct ToDebug<'a, 'b: 'a>(&'a Source<'b>);

        impl<'a, 'b: 'a> fmt::Display for ToDebug<'a, 'b> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Debug::fmt(self.0, f)
            }
        }

        ToDebug(self)
    }

    pub fn render_json<'b>(&'b self) -> impl fmt::Display + 'b {
        struct ToJson<'a, 'b: 'a>(&'a Source<'b>);

        impl<'a, 'b: 'a> fmt::Display for ToJson<'a, 'b> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                sval_json::to_fmt(f, self.0).map_err(|_| fmt::Error)?;
                Ok(())
            }
        }

        ToJson(self)
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

impl<'a> fmt::Debug for Source<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.sorted_key_values, f)
    }
}

impl<'a> Value for Source<'a> {
    fn stream(&self, stream: &mut value::Stream) -> value::Result {
        stream.map_begin(Some(self.sorted_key_values.len()))?;

        for (k, v) in self.sorted_key_values {
            stream.map_key(k)?;
            stream.map_value(v)?;
        }

        stream.map_end()
    }
}

#[cfg(feature = "serde")]
impl<'a> Serialize for Source<'a> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = s.serialize_map(Some(self.sorted_key_values.len()))?;

        for (k, v) in self.sorted_key_values {
            map.serialize_entry(k, v)?;
        }

        map.end()
    }
}
