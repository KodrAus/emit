use std::{error, fmt};

pub struct Error(Box<dyn error::Error + Send + Sync>);

impl Error {
    pub(crate) fn new(e: impl error::Error + Send + Sync + 'static) -> Self {
        Error(Box::new(e))
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.0.source()
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
