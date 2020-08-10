use std::{error, fmt, time::SystemTime};

#[doc(inline)]
pub use log::{kv, Level};

use self::kv::{Key, ToValue, Value};

pub const ERROR_KEY: &'static str = "error";
pub const TIMESTAMP_KEY: &'static str = "timestamp";

pub struct Record<'a> {
    source: Source<'a>,
    inner: &'a log::Record<'a>,
}

struct Source<'a> {
    timestamp: Timestamp<'a>,
    template: Option<Template<'a>>,
    error: Option<SourceError<'a>>,
    inner: &'a dyn kv::Source,
}

impl<'a> Record<'a> {
    pub fn from_log(record: &'a log::Record<'a>) -> Self {
        let source = Source::from_log(record.key_values());

        Record {
            inner: record,
            source,
        }
    }

    pub fn to_log(&self) -> log::Record {
        let mut record = self.inner.to_builder();

        record.key_values(&self.source).build()
    }

    pub fn args(&self) -> &fmt::Arguments {
        &self.inner.args()
    }

    pub fn template(&self) -> Option<&Template> {
        self.source.template.as_ref()
    }

    pub fn timestamp(&self) -> &Timestamp {
        &self.source.timestamp
    }

    pub fn error(&self) -> Option<&dyn error::Error> {
        if let Some(ref err) = self.source.error {
            Some(err)
        } else {
            None
        }
    }
}

impl<'a> Source<'a> {
    fn from_log(source: &'a dyn kv::Source) -> Self {
        let timestamp = if let Some(timestamp) = source.get(Key::from_str(TIMESTAMP_KEY)) {
            Timestamp(Captured::Captured(timestamp))
        } else {
            Timestamp(Captured::Provided(humantime::format_rfc3339_nanos(
                SystemTime::now(),
            )))
        };

        Source {
            timestamp,
            template: None,
            error: None,
            inner: source,
        }
    }
}

impl<'a> kv::Source for Source<'a> {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn kv::Visitor<'kvs>) -> Result<(), kv::Error> {
        if let Captured::Provided(ref ts) = self.timestamp.0 {
            visitor.visit_pair(Key::from_str(TIMESTAMP_KEY), Value::from_display(ts))?;
        }

        self.inner.visit(visitor)
    }

    fn get<'v>(&'v self, key: Key) -> Option<Value<'v>> {
        match key.as_str() {
            TIMESTAMP_KEY => Some(self.timestamp.to_value()),
            _ => self.inner.get(key),
        }
    }
}

pub struct Timestamp<'a>(Captured<'a, humantime::Rfc3339Timestamp>);

impl<'a> ToValue for Timestamp<'a> {
    fn to_value(&self) -> Value {
        match self.0 {
            Captured::Captured(ref v) => v.to_value(),
            Captured::Provided(ref ts) => Value::from_display(ts),
        }
    }
}

pub struct Template<'a>(Captured<'a, &'a str>);

impl<'a> ToValue for Template<'a> {
    fn to_value(&self) -> Value {
        match self.0 {
            Captured::Captured(ref v) => v.to_value(),
            Captured::Provided(ref template) => template.to_value(),
        }
    }
}

impl<'a> Template<'a> {
    pub fn render(&self, fmt: impl FnMut(fmt::Write, &str, Value) -> fmt::Result) {
        unimplemented!("iterate through template parts and render using `source.get(label)`")
    }
}

struct SourceError<'a>(Captured<'a, &'a (dyn error::Error + 'static)>);

impl<'a> ToValue for SourceError<'a> {
    fn to_value(&self) -> Value {
        match self.0 {
            Captured::Captured(ref v) => v.to_value(),
            Captured::Provided(ref err) => Value::from_display(err),
        }
    }
}

impl<'a> error::Error for SourceError<'a> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        if let Captured::Provided(err) = self.0 {
            Some(err)
        } else {
            None
        }
    }
}

impl<'a> fmt::Debug for SourceError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Captured::Captured(ref v) => fmt::Debug::fmt(v, f),
            Captured::Provided(ref err) => fmt::Debug::fmt(err, f),
        }
    }
}

impl<'a> fmt::Display for SourceError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Captured::Captured(ref v) => fmt::Display::fmt(v, f),
            Captured::Provided(ref err) => fmt::Display::fmt(err, f),
        }
    }
}

enum Captured<'a, T> {
    Provided(T),
    Captured(Value<'a>),
}
