use crate::{kvs::KeyValues, std::fmt, template::Template};

#[cfg(feature = "sval")]
use sval_lib::value::{self, Value};

#[cfg(feature = "serde")]
use serde_lib::ser::{Serialize, SerializeMap, Serializer};

pub struct Record<'a> {
    pub kvs: KeyValues<'a>,
    pub template: Template<'a>,
}

impl<'a> Record<'a> {
    pub fn render_msg<'b>(&'b self) -> impl fmt::Display + 'b {
        self.template.render(fv_template::rt::Context::new().fill(
            move |write: &mut fmt::Formatter, label| {
                self.kvs
                    .get(label)
                    .map(|value| fmt::Display::fmt(&value, write))
            },
        ))
    }

    pub fn render_template<'b>(&'b self) -> impl fmt::Display + 'b {
        self.template.render(Default::default())
    }
}

impl<'a> fmt::Display for Record<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.render_msg().fmt(f)
    }
}

#[cfg(feature = "sval")]
impl<'a> Value for Record<'a> {
    fn stream(&self, stream: &mut value::Stream) -> value::Result {
        stream.map_begin(Some(self.kvs.sorted_key_values.len() + 2))?;

        stream.map_key("message")?;
        stream.map_value(format_args!("{}", self.render_msg()))?;

        stream.map_key("template")?;
        stream.map_value(format_args!("{}", self.render_template()))?;

        for (k, v) in self.kvs.sorted_key_values {
            stream.map_key(k)?;
            stream.map_value(v)?;
        }

        stream.map_end()
    }
}

#[cfg(feature = "serde")]
impl<'a> Serialize for Record<'a> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = s.serialize_map(Some(self.kvs.sorted_key_values.len() + 2))?;

        map.serialize_entry("message", format_args!("{}", self.render_msg()))?;
        map.serialize_entry("template", format_args!("{}", self.render_template()))?;

        for (k, v) in self.kvs.sorted_key_values {
            map.serialize_entry(k, v)?;
        }

        map.end()
    }
}
