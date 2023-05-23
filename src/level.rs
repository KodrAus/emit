use core::fmt;

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

impl Default for Level {
    fn default() -> Self {
        Level::Info
    }
}
