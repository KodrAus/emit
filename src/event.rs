use crate::std::fmt;

#[cfg(feature = "std")]
use crate::std::time::Duration;

use fv_template::rt::Context;
use value_bag::ValueBag;

pub struct Event<'a>(pub(crate) &'a crate::rt::__private::RawEvent<'a>);

impl<'a> Event<'a> {
    #[cfg(feature = "std")]
    pub fn ts(&self) -> Timestamp {
        Timestamp(self.0.ts.0)
    }

    pub fn lvl(&self) -> Level {
        match self.0.lvl {
            emit_rt::__private::RawLevel::DEBUG => Level::Debug,
            emit_rt::__private::RawLevel::INFO => Level::Info,
            emit_rt::__private::RawLevel::WARN => Level::Warn,
            emit_rt::__private::RawLevel::ERROR => Level::Error,
            _ => Level::Info,
        }
    }

    pub fn msg(&self) -> Template<'a> {
        self.tpl().msg()
    }

    pub fn tpl(&self) -> Template<'a> {
        Template {
            event: self.0,
            style: Default::default(),
            props: Default::default(),
        }
    }

    pub fn props(&self) -> Properties<'a> {
        Properties { event: self.0 }
    }
}

#[cfg(feature = "std")]
pub struct Timestamp(Duration);

#[cfg(feature = "std")]
impl Timestamp {
    pub fn elapsed_since_unix_epoch(&self) -> Duration {
        self.0
    }
}

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
    pub fn get(&self, key: impl AsRef<str>) -> Option<Value<'a>> {
        self.event.get(key).map(|value| Value(value.by_ref()))
    }

    #[cfg(feature = "std")]
    pub fn err(&self) -> Option<Error<'a>> {
        self.event
            .get(crate::rt::__private::WELL_KNOWN_ERR_KEY)
            .map(|err| Error(err.by_ref()))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'a str, Value<'a>)> {
        self.event
            .props
            .iter()
            .map(|(k, v)| (*k, Value(v.by_ref())))
    }
}

pub struct Value<'a>(ValueBag<'a>);

impl<'a> Value<'a> {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }

    pub fn to_i64(&self) -> Option<i64> {
        self.0.to_i64()
    }
}

impl<'a> fmt::Debug for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl<'a> fmt::Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(feature = "sval")]
impl<'a> sval::Value for Value<'a> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        self.0.stream(stream)
    }
}

#[cfg(feature = "serde")]
impl<'a> serde::Serialize for Value<'a> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "std")]
pub struct Error<'a>(ValueBag<'a>);

#[cfg(feature = "std")]
impl<'a> Error<'a> {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }
}

#[cfg(feature = "std")]
impl<'a> fmt::Debug for Error<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

#[cfg(feature = "std")]
impl<'a> fmt::Display for Error<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(feature = "std")]
impl<'a> crate::std::error::Error for Error<'a> {
    fn source(&self) -> Option<&(dyn crate::std::error::Error + 'static)> {
        self.0.to_borrowed_error().and_then(|err| err.source())
    }
}

pub struct Template<'a> {
    event: &'a crate::rt::__private::RawEvent<'a>,
    style: TemplateStyle,
    props: bool,
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
    pub fn msg(self) -> Self {
        Template {
            event: self.event,
            style: self.style,
            props: true,
        }
    }

    pub fn braced(self) -> Self {
        Template {
            event: self.event,
            props: self.props,
            style: TemplateStyle::Braced,
        }
    }
}

impl<'a> fmt::Display for Template<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ctxt = Context::new().fill(|f, label| {
            self.props
                .then(|| self.event.get(label))
                .and_then(|value| value)
                .map(|value| fmt::Display::fmt(value, f))
        });

        match self.style {
            TemplateStyle::Tilde => fmt::Display::fmt(&self.event.tpl.render(ctxt), f),
            TemplateStyle::Braced => fmt::Display::fmt(
                &self
                    .event
                    .tpl
                    .render(ctxt.missing(|f, label| write!(f, "{{{}}}", label))),
                f,
            ),
        }
    }
}
