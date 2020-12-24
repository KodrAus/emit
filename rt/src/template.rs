use crate::{kvs::KeyValues, std::fmt};

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
