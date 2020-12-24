use crate::{kvs::KeyValues, std::fmt};

use sval::value::{self, Value};

#[cfg(feature = "serde")]
use serde_lib::ser::{Serialize, Serializer};

pub use fv_template::rt::*;

pub trait TemplateRender<'a> {
    fn render_template<'b>(&'b self) -> RenderTemplate<'a, 'b>;
    fn render_kvs<'b>(&'b self, kvs: KeyValues<'b>) -> RenderKeyValues<'a, 'b>;
}

impl<'a> TemplateRender<'a> for Template<'a> {
    fn render_template<'b>(&'b self) -> RenderTemplate<'a, 'b> {
        RenderTemplate(self)
    }

    fn render_kvs<'b>(&'b self, kvs: KeyValues<'b>) -> RenderKeyValues<'a, 'b> {
        RenderKeyValues(self, kvs)
    }
}

pub struct RenderTemplate<'a, 'b>(&'b Template<'a>);
pub struct RenderKeyValues<'a, 'b>(&'b Template<'a>, KeyValues<'b>);

impl<'a, 'b> fmt::Display for RenderTemplate<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.render(Default::default()).fmt(f)
    }
}

impl<'a, 'b> Value for RenderTemplate<'a, 'b> {
    fn stream(&self, stream: &mut value::Stream) -> value::Result {
        stream.display(self)
    }
}

#[cfg(feature = "serde")]
impl<'a, 'b> Serialize for RenderTemplate<'a, 'b> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'a, 'b> fmt::Display for RenderKeyValues<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.render(fv_template::rt::Context::new().fill(
            move |write: &mut fmt::Formatter, label| {
                self.1
                    .get(label)
                    .map(|value| fmt::Display::fmt(&value, write))
            },
        )).fmt(f)
    }
}

impl<'a, 'b> Value for RenderKeyValues<'a, 'b> {
    fn stream(&self, stream: &mut value::Stream) -> value::Result {
        stream.display(self)
    }
}

#[cfg(feature = "serde")]
impl<'a, 'b> Serialize for RenderKeyValues<'a, 'b> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}
