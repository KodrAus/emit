use crate::std::fmt;

use fv_template::rt::Context;
pub use value_bag::ValueBag;

pub struct Event<'a>(pub(crate) &'a crate::rt::__private::RawEvent<'a>);

impl<'a> Event<'a> {
    pub fn timestamp(&self) -> Timestamp {
        Timestamp
    }

    pub fn level(&self) -> Level {
        match self.0.level {
            emit_rt::__private::RawLevel::DEBUG => Level::Debug,
            emit_rt::__private::RawLevel::INFO => Level::Info,
            emit_rt::__private::RawLevel::WARN => Level::Warn,
            emit_rt::__private::RawLevel::ERROR => Level::Error,
            _ => Level::Info,
        }
    }

    pub fn message<'b>(&'b self) -> Template<'b> {
        self.template().message()
    }

    pub fn template<'b>(&'b self) -> Template<'b> {
        Template {
            event: self.0,
            style: Default::default(),
            properties: Default::default(),
        }
    }

    pub fn properties<'b>(&'b self) -> Properties<'b> {
        Properties { event: self.0 }
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
    event: &'a crate::rt::__private::RawEvent<'a>,
}

impl<'a> Properties<'a> {
    pub fn get<'b>(&'b self, key: impl AsRef<str>) -> Option<ValueBag<'b>> {
        self.event.get(key).map(|value| value.by_ref())
    }

    pub fn iter<'b>(&'b self) -> impl Iterator<Item = (&'b str, ValueBag<'b>)> {
        self.event.properties.iter().map(|(k, v)| (*k, v.by_ref()))
    }
}

pub struct Template<'a> {
    event: &'a crate::rt::__private::RawEvent<'a>,
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
            event: self.event,
            style: self.style,
            properties: true,
        }
    }

    pub fn braced(self) -> Self {
        Template {
            event: self.event,
            properties: self.properties,
            style: TemplateStyle::Braced,
        }
    }
}

impl<'a> fmt::Display for Template<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ctxt = Context::new().fill(|f, label| {
            self.properties
                .then(|| self.event.get(label))
                .and_then(|value| value)
                .map(|value| fmt::Display::fmt(value, f))
        });

        match self.style {
            TemplateStyle::Tilde => fmt::Display::fmt(&self.event.template.render(ctxt), f),
            TemplateStyle::Braced => fmt::Display::fmt(
                &self
                    .event
                    .template
                    .render(ctxt.missing(|f, label| write!(f, "{{{}}}", label))),
                f,
            ),
        }
    }
}
