use std::{error, fmt};

use log::kv;

/**
Similar to `log`'s `ToValue` trait, but with a generic pivot parameter.

The parameter lets us choose the way values are captured, since many
types can be captured in lots of different ways.
*/
pub trait Capture<T: ?Sized> {
    fn capture(&self) -> kv::Value;
}

/**
The default capturing method is with the `Value::capture_display` method.
*/
pub type WithDefault = CaptureDisplay;

extern "C" {
    pub type CaptureDisplay;
    pub type CaptureDebug;
    pub type CaptureError;
}

impl<T> Capture<CaptureDisplay> for T
where
    T: fmt::Display + 'static,
{
    default fn capture(&self) -> kv::Value {
        kv::Value::capture_display(self)
    }
}

impl Capture<CaptureDisplay> for dyn fmt::Display {
    fn capture(&self) -> kv::Value {
        kv::Value::from(self)
    }
}

impl<T> Capture<CaptureDebug> for T
where
    T: fmt::Debug + 'static,
{
    default fn capture(&self) -> kv::Value {
        kv::Value::capture_debug(self)
    }
}

impl Capture<CaptureDebug> for dyn fmt::Debug {
    fn capture(&self) -> kv::Value {
        kv::Value::from(self)
    }
}

impl<T> Capture<CaptureError> for T
where
    T: error::Error + 'static,
{
    default fn capture(&self) -> kv::Value {
        kv::Value::capture_error(self)
    }
}

impl Capture<CaptureError> for dyn error::Error {
    fn capture(&self) -> kv::Value {
        kv::Value::from(self)
    }
}

impl<T: ?Sized> Capture<T> for str {
    fn capture(&self) -> kv::Value {
        kv::Value::from(self)
    }
}

/**
An API to the specialized `Capture` trait for consuming in a macro.

This trait is a bit weird looking. It's shaped to serve a few purposes
in the private macro API:

- It supports auto-ref so that something like a `u64` or `&str` can be
captured using the same `x.method()` syntax.
- It uses `Self` bounds on each method, and is unconditionally implemented
so that when a bound isn't satisfied we get a more accurate type error.
- It uses clumsily uglified names that are unlikely to clash in non-hygeinic
contexts. (We're expecting non-hygeinic spans to support value interpolation).
*/
pub trait __PrivateLogCapture {
    fn __private_log_capture_with_default(&self) -> kv::Value
    where
        Self: Capture<WithDefault>,
    {
        Capture::capture(self)
    }

    fn __private_log_capture_from_display(&self) -> kv::Value
    where
        Self: Capture<CaptureDisplay>,
    {
        Capture::capture(self)
    }

    fn __private_log_capture_from_debug(&self) -> kv::Value
    where
        Self: Capture<CaptureDebug>,
    {
        Capture::capture(self)
    }

    fn __private_log_capture_from_error(&self) -> kv::Value
    where
        Self: Capture<CaptureError>,
    {
        Capture::capture(self)
    }
}

impl<T: ?Sized> __PrivateLogCapture for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    #[test]
    fn capture_default() {
        struct SomeType;

        impl fmt::Display for SomeType {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "some type")
            }
        }

        // Capture an arbitrary `Display`
        let _ = SomeType.__private_log_capture_with_default();

        // Capture a structured number
        assert_eq!(
            Some(42u64),
            42u64.__private_log_capture_with_default().to_u64()
        );

        // Capture a borrowed (non-static) string
        let v: &str = &String::from("a string");
        assert_eq!(
            Some("a string"),
            v.__private_log_capture_with_default().to_borrowed_str()
        );

        // Capture a value with parens
        let _ = (SomeType).__private_log_capture_with_default();

        // Capture and borrow a string as an expression
        let v = SomeType;
        match (
            (v).__private_log_capture_with_default(),
            (String::from("a string")).__private_log_capture_with_default(),
        ) {
            (a, b) => {
                let _ = a;
                let _ = b;
            }
        }
        let _ = v;
    }

    #[test]
    fn capture_display() {
        struct SomeType;

        impl fmt::Display for SomeType {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "some type")
            }
        }

        // Capture an arbitrary `Display`
        let _ = SomeType.__private_log_capture_from_display();

        // Capture a `&dyn Display`
        let v: &dyn fmt::Display = &SomeType;
        let _ = v.__private_log_capture_from_display();
    }

    #[test]
    fn capture_debug() {
        struct SomeType;

        impl fmt::Debug for SomeType {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "some type")
            }
        }

        // Capture an arbitrary `Debug`
        let _ = SomeType.__private_log_capture_from_debug();

        // Capture a `&dyn Debug`
        let v: &dyn fmt::Debug = &SomeType;
        let _ = v.__private_log_capture_from_debug();
    }

    #[test]
    fn capture_error() {
        use std::io;

        // Capture an arbitrary `Error`
        let err = io::Error::from(io::ErrorKind::Other);
        assert!(err.__private_log_capture_from_error().to_error().is_some());

        // Capture a `&dyn Error`
        let err: &dyn error::Error = &err;
        assert!(err.__private_log_capture_from_error().to_error().is_some());
    }
}
