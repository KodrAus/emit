use crate::value::{ToValue, Value};
use core::{fmt, str::FromStr};

#[derive(Clone, Copy)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Debug for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Level::Info => "INFO",
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Debug => "DEBUG",
        })
    }
}

impl FromStr for Level {
    type Err = ParseLevelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

pub struct ParseLevelError {}

impl Default for Level {
    fn default() -> Self {
        Level::Info
    }
}

impl ToValue for Level {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> Value<'v> {
    pub fn to_level(&self) -> Option<Level> {
        self.downcast_ref::<Level>()
            .copied()
            .or_else(|| self.parse())
    }
}
