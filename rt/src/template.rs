use crate::{source::Source, std::fmt};

pub use fv_template::rt::*;

pub trait TemplateRender<'a> {
    fn render_template<'b>(&'b self) -> FmtRenderTemplate<'a, 'b>;
    fn render_source<'b>(&'b self, source: Source<'b>) -> FmtRenderSource<'a, 'b>;
}

impl<'a> TemplateRender<'a> for Template<'a> {
    fn render_template<'b>(&'b self) -> FmtRenderTemplate<'a, 'b> {
        FmtRenderTemplate(self)
    }

    fn render_source<'b>(&'b self, source: Source<'b>) -> FmtRenderSource<'a, 'b> {
        FmtRenderSource(self, source)
    }
}

pub struct FmtRenderTemplate<'a, 'b>(&'b Template<'a>);
pub struct FmtRenderSource<'a, 'b>(&'b Template<'a>, Source<'b>);

impl<'a, 'b> fmt::Display for FmtRenderTemplate<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.render(Default::default()).fmt(f)
    }
}

impl<'a, 'b> fmt::Display for FmtRenderSource<'a, 'b> {
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
