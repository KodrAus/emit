use crate::{
    std::fmt,
    source::Source,
};

pub struct Template<'a>(pub fv_template::rt::Template<'a>);

impl<'a> Template<'a> {
    pub fn render_template<'b>(&'b self) -> impl fmt::Display + 'b {
        self.0.render(Default::default())
    }

    pub fn render_source<'b>(&'b self, source: Source<'b>) -> impl fmt::Display + 'b {
        self.0.render(fv_template::rt::Context::new().fill(
            move |write: &mut fmt::Formatter, label| {
                source
                    .get(label)
                    .map(|value| fmt::Display::fmt(&value, write))
            },
        ))
    }
}
