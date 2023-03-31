use crate::std::fmt;

use fv_template::rt::Context;
pub use value_bag::ValueBag;

pub struct Event<'a>(pub(crate) &'a crate::rt::__private::Record<'a>);

impl<'a> Event<'a> {
    pub fn timestamp(&self) -> Timestamp {
        Timestamp
    }

    pub fn level(&self) -> Level {
        match self.0.level {
            emit_rt::__private::Level::DEBUG => Level::Debug,
            emit_rt::__private::Level::INFO => Level::Info,
            emit_rt::__private::Level::WARN => Level::Warn,
            emit_rt::__private::Level::ERROR => Level::Error,
            _ => Level::Info,
        }
    }

    pub fn message<'b>(&'b self) -> Template<'b> {
        self.template().message()
    }

    pub fn template<'b>(&'b self) -> Template<'b> {
        Template {
            record: self.0,
            style: Default::default(),
            properties: Default::default(),
        }
    }

    pub fn properties<'b>(&'b self) -> Properties<'b> {
        Properties { record: self.0 }
    }
}

#[derive(Debug)]
pub struct Timestamp;

#[derive(Debug)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
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

pub struct Template<'a> {
    record: &'a crate::rt::__private::Record<'a>,
    style: TemplateStyle,
    properties: bool,
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
    pub fn message(self) -> Self {
        Template {
            record: self.record,
            style: self.style,
            properties: true,
        }
    }

    pub fn braced(self) -> Self {
        Template {
            record: self.record,
            properties: self.properties,
            style: TemplateStyle::Braced,
        }
    }
}

impl<'a> fmt::Display for Template<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ctxt = Context::new().fill(|f, label| {
            self.properties
                .then(|| self.record.get(label))
                .and_then(|value| value)
                .map(|value| fmt::Display::fmt(value, f))
        });

        match self.style {
            TemplateStyle::Tilde => fmt::Display::fmt(&self.record.template.render(ctxt), f),
            TemplateStyle::Braced => fmt::Display::fmt(
                &self
                    .record
                    .template
                    .render(ctxt.missing(|f, label| write!(f, "{{{}}}", label))),
                f,
            ),
        }
    }
}
