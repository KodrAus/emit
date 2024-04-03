use std::{error, fmt};

pub struct Error {
    msg: String,
    cause: Box<dyn error::Error + Send + Sync>,
}

impl Error {
    pub(crate) fn new(
        msg: impl fmt::Display,
        e: impl error::Error + Send + Sync + 'static,
    ) -> Self {
        Error {
            msg: msg.to_string(),
            cause: Box::new(e),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&*self.cause)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.msg, f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.msg, f)
    }
}
