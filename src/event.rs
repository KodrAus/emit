use crate::std::fmt;

#[cfg(feature = "std")]
use crate::std::{
    error,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use fv_template::rt::Context;
use value_bag::ValueBag;

/**
An event is a semantic record of some notable change in a system.

Events combine a timestamp, a message template, and associated structured properties.
*/
pub struct Event<'a>(pub(crate) &'a crate::rt::__private::RawEvent<'a>);

impl<'a> Event<'a> {
    /**
    Get the time at which the event occurred.
    */
    #[cfg(feature = "std")]
    pub fn ts(&self) -> Timestamp {
        Timestamp(self.0.ts.0)
    }

    /**
    Get an indicator of the kind of event that occurred.
    */
    pub fn lvl(&self) -> Level {
        match self.0.lvl {
            emit_rt::__private::RawLevel::DEBUG => Level::Debug,
            emit_rt::__private::RawLevel::INFO => Level::Info,
            emit_rt::__private::RawLevel::WARN => Level::Warn,
            emit_rt::__private::RawLevel::ERROR => Level::Error,
            _ => Level::Info,
        }
    }

    /**
    Get a description of the event, intended for end-users.
    */
    pub fn msg(&self) -> Template<'a> {
        self.tpl().msg()
    }

    /**
    Get the template of the event.

    This template can either be rendered using the event's properties,
    or converted into a textual representation with placeholders for
    property values.
    */
    pub fn tpl(&self) -> Template<'a> {
        Template {
            event: self.0,
            style: Default::default(),
            props: Default::default(),
        }
    }

    /**
    Get the properties captured with the event.
    */
    pub fn props(&self) -> Properties<'a> {
        Properties(self.0)
    }
}

/**
The time at which an event occurred.
*/
#[cfg(feature = "std")]
pub struct Timestamp(Duration);

#[cfg(feature = "std")]
impl Timestamp {
    /**
    Convert the timestamp into a standard representation.
    */
    pub fn to_system_time(&self) -> SystemTime {
        UNIX_EPOCH + self.0
    }
}

/**
A course-grained category for the event.

Levels provide some quick guage of the notability of a particular event.
They're a standard concept shared by many tools and a useful static filter.
*/
#[derive(Debug)]
pub enum Level {
    /**
    A weakly informative event that may be useful for debugging.
    */
    Debug,
    /**
    An informative event.
    */
    Info,
    /**
    A weakly erroneous event that was recovered from.
    */
    Warn,
    /**
    An erroneous event.
    */
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/**
The structured properties captured with an event.
*/
pub struct Properties<'a>(&'a crate::rt::__private::RawEvent<'a>);

impl<'a> Properties<'a> {
    /**
    Get a property by name.
    */
    pub fn get(&self, key: impl AsRef<str>) -> Option<Value<'a>> {
        self.0.get(key).map(|value| Value(value.by_ref()))
    }

    /**
    Get the semantic `err` property.

    The error can be treated like a regular [`std::error::Error`].
    If it was originally captured as an error then its backtrace and source
    chain can also be accessed.
    */
    #[cfg(feature = "std")]
    pub fn err(&self) -> Option<Error<'a>> {
        self.0
            .get(crate::well_known::ERR)
            .map(|err| Error(err.by_ref()))
    }
}

impl<'a> IntoIterator for Properties<'a> {
    type Item = Property<'a>;
    type IntoIter = PropertiesIntoIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        PropertiesIntoIter(self.0.props.iter())
    }
}

/**
An iterator over properties on an event.
*/
pub struct PropertiesIntoIter<'a>(
    <emit_rt::__private::RawProperties<'a> as IntoIterator>::IntoIter,
);

impl<'a> Iterator for PropertiesIntoIter<'a> {
    type Item = Property<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| Property(*k, v))
    }
}

/**
A key-value pair.
*/
pub struct Property<'a>(&'a str, &'a ValueBag<'a>);

impl<'a> Property<'a> {
    /**
    Get the key name of this property.
    */
    pub fn key(&self) -> &'a str {
        self.0
    }

    /**
    Get the value of this property.
    */
    pub fn value(&self) -> Value<'a> {
        Value(self.1.by_ref())
    }

    /**
    Whether or not this property is the semantic `err`.
    */
    #[cfg(feature = "std")]
    pub fn is_err(&self) -> bool {
        self.0 == crate::well_known::ERR
    }

    /**
    Try get the value of this property as the semantic `err`.

    This method will return `None` if the property is not `err`.
    */
    #[cfg(feature = "std")]
    pub fn err(&self) -> Option<Error<'a>> {
        if self.is_err() {
            Some(Error(self.1.by_ref()))
        } else {
            None
        }
    }
}

/**
An individual property value.
*/
pub struct Value<'a>(ValueBag<'a>);

impl<'a> Value<'a> {
    /**
    Attempt to downcast the value to some concrete type.
    */
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }

    /**
    Try convert the value into a signed 64bit integer.
    */
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

/**
The semantic `err` property value.
*/
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
impl<'a> error::Error for Error<'a> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.0.to_borrowed_error().and_then(|err| err.source())
    }
}

/**
A template is a textual message with holes to format properties into.

Templates are lazy, they can be evaluated by formatting them either
using their `Debug` or `Display` implementations.
*/
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
    /**
    Render properties into the event.
    */
    pub fn msg(self) -> Self {
        Template {
            event: self.event,
            style: self.style,
            props: true,
        }
    }

    /**
    Render the holes in the template using braces like `{key}`.
    */
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
