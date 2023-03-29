use crate::std::fmt;

use fv_template::rt::Context;
pub use value_bag::ValueBag;

pub struct Record<'a>(pub(crate) &'a crate::rt::__private::Record<'a>);

impl<'a> Record<'a> {
    pub fn message<'b>(&'b self) -> Message<'b> {
        Message { record: self.0 }
    }

    pub fn template<'b>(&'b self) -> Template<'b> {
        Template {
            record: self.0,
            style: Default::default(),
        }
    }

    pub fn properties<'b>(&'b self) -> Properties<'b> {
        Properties { record: self.0 }
    }
}

pub struct Properties<'a> {
    record: &'a crate::rt::__private::Record<'a>,
}

impl<'a> Properties<'a> {
    pub fn get<'b>(&'b self, key: impl AsRef<str>) -> Option<ValueBag<'b>> {
        self.record.get(key).map(|value| value.by_ref())
    }

    pub fn iter<'b>(&'b self) -> impl Iterator<Item = (&'b str, ValueBag<'b>)> {
        self.record.kvs.iter().map(|(k, v)| (*k, v.by_ref()))
    }
}

pub struct Message<'a> {
    record: &'a crate::rt::__private::Record<'a>,
}

impl<'a> fmt::Display for Message<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.record, f)
    }
}

pub struct Template<'a> {
    record: &'a crate::rt::__private::Record<'a>,
    style: TemplateStyle,
}

enum TemplateStyle {
    Tilde,
    Braced,
}

impl Default for TemplateStyle {
    fn default() -> Self {
        TemplateStyle::Tilde
    }
}

impl<'a> Template<'a> {
    pub fn braced(self) -> Self {
        Template {
            record: self.record,
            style: TemplateStyle::Braced,
        }
    }
}

impl<'a> fmt::Display for Template<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.style {
            TemplateStyle::Tilde => {
                fmt::Display::fmt(&self.record.template.render(Default::default()), f)
            }
            TemplateStyle::Braced => fmt::Display::fmt(
                &self
                    .record
                    .template
                    .render(Context::new().missing(|f, label| write!(f, "{{{}}}", label))),
                f,
            ),
        }
    }
}
