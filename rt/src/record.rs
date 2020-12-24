use crate::{kvs::KeyValues, template::{Template, TemplateRender}, std::fmt};

use sval::value::{self, Value};

#[cfg(feature = "serde")]
use serde_lib::ser::{Serialize, Serializer, SerializeMap};

pub struct Record<'a> {
    pub kvs: KeyValues<'a>,
    pub template: Template<'a>,
}

impl<'a> fmt::Display for Record<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.template.render_kvs(self.kvs).fmt(f)
    }
}

impl<'a> Value for Record<'a> {
    fn stream(&self, stream: &mut value::Stream) -> value::Result {
        stream.map_begin(Some(self.kvs.sorted_key_values.len() + 2))?;

        stream.map_key("msg")?;
        stream.map_value(self.template.render_kvs(self.kvs))?;

        stream.map_key("template")?;
        stream.map_value(self.template.render_template())?;

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

        map.serialize_entry("msg", &self.template.render_kvs(self.kvs))?;
        map.serialize_entry("template", &self.template.render_template())?;

        for (k, v) in self.kvs.sorted_key_values {
            map.serialize_entry(k, v)?;
        }

        map.end()
    }
}
