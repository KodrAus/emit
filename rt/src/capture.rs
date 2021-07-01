use crate::{std::fmt, value::ValueBag};

#[cfg(feature = "std")]
use crate::std::error::Error;

#[cfg(feature = "serde")]
use serde_lib::Serialize;

use sval::value::Value;

/**
Similar to `log`'s `ToValue` trait, but with a generic pivot parameter.

The parameter lets us choose the way values are captured, since many
types can be captured in lots of different ways.
*/
pub trait Capture<T: ?Sized> {
    fn capture(&self) -> ValueBag;
}

/**
The default capturing method is with the `Value::capture_display` method.
*/
pub type WithDefault = CaptureDisplay;

pub enum CaptureDisplay {}
pub enum CaptureDebug {}
pub enum CaptureSval {}
pub enum CaptureSerde {}
pub enum CaptureError {}

impl<'a, T> Capture<CaptureDisplay> for &'a T
where
    T: fmt::Display + ?Sized + 'static,
{
    fn capture(&self) -> ValueBag {
        ValueBag::try_capture(*self).unwrap_or_else(|| ValueBag::from_display(self))
    }
}

impl<'a, T> Capture<CaptureDebug> for &'a T
where
    T: fmt::Debug + ?Sized + 'static,
{
    fn capture(&self) -> ValueBag {
        ValueBag::try_capture(*self).unwrap_or_else(|| ValueBag::from_debug(self))
    }
}

impl<'a, T> Capture<CaptureSval> for &'a T
where
    T: Value + ?Sized + 'static,
{
    fn capture(&self) -> ValueBag {
        ValueBag::try_capture(*self).unwrap_or_else(|| ValueBag::from_sval1(self))
    }
}

#[cfg(feature = "serde")]
impl<'a, T> Capture<CaptureSerde> for &'a T
where
    T: Serialize + 'static,
{
    fn capture(&self) -> ValueBag {
        ValueBag::try_capture(self).unwrap_or_else(|| ValueBag::from_serde1(self))
    }
}

#[cfg(feature = "std")]
impl<'a, T> Capture<CaptureError> for &'a T
where
    T: Error + 'static,
{
    fn capture(&self) -> ValueBag {
        ValueBag::from_dyn_error(*self)
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
pub trait __PrivateCapture {
    fn __private_capture_as_default(&self) -> ValueBag
    where
        Self: Capture<WithDefault>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_display(&self) -> ValueBag
    where
        Self: Capture<CaptureDisplay>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_debug(&self) -> ValueBag
    where
        Self: Capture<CaptureDebug>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_sval(&self) -> ValueBag
    where
        Self: Capture<CaptureSval>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_serde(&self) -> ValueBag
    where
        Self: Capture<CaptureSerde>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_error(&self) -> ValueBag
    where
        Self: Capture<CaptureError>,
    {
        Capture::capture(self)
    }
}

impl<'a, T: ?Sized> __PrivateCapture for &'a T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::std::{fmt, string::String};

    #[test]
    fn capture_default() {
        struct SomeType;

        impl fmt::Display for SomeType {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "some type")
            }
        }

        // Capture an arbitrary `Display`
        let _ = (&SomeType).__private_capture_as_default();

        // Capture a structured number
        assert_eq!(Some(42u64), (&42u64).__private_capture_as_default().to_u64());

        // Capture a borrowed (non-static) string
        let v: &str = &String::from("a string");
        assert_eq!(
            Some("a string"),
            (&v).__private_capture_as_default().to_borrowed_str()
        );

        // Capture a value with parens
        let _ = (&(SomeType)).__private_capture_as_default();

        // Capture and borrow a string as an expression
        let v = SomeType;
        match (
            (&v).__private_capture_as_default(),
            (&String::from("a string")).__private_capture_as_default(),
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
        let _ = (&SomeType).__private_capture_as_display();

        // Capture a `&dyn Display`
        let v: &dyn fmt::Display = &SomeType;
        let _ = (&v).__private_capture_as_display();
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
        let _ = (&SomeType).__private_capture_as_debug();

        // Capture a `&dyn Debug`
        let v: &dyn fmt::Debug = &SomeType;
        let _ = (&v).__private_capture_as_debug();
    }

    #[test]
    fn capture_sval() {
        use sval::value::{self, Value};

        struct Map;

        impl Value for Map {
            fn stream(&self, stream: &mut value::Stream) -> value::Result {
                stream.map_begin(Some(2))?;

                stream.map_key("a")?;
                stream.map_value(42)?;

                stream.map_key("b")?;
                stream.map_value(17)?;

                stream.map_end()
            }
        }

        let map = Map;

        // Capture an arbitrary `Value`
        let _ = (&map).__private_capture_as_sval();

        // Capture a `&dyn Value`
        let v: &dyn Value = &map;
        let _ = (&v).__private_capture_as_sval();
    }

    #[test]
    #[cfg(feature = "serde")]
    fn capture_serde() {
        let tuple = (1, 2, 3, 4, 5);

        let _ = (&tuple).__private_capture_as_serde();
    }

    #[test]
    #[cfg(feature = "std")]
    fn capture_error() {
        use crate::std::io;

        // Capture an arbitrary `Error`
        let err = io::Error::from(io::ErrorKind::Other);
        assert!((&err).__private_capture_as_error().to_borrowed_error().is_some());

        // Capture a `&dyn Error`
        let err: &dyn Error = &err;
        assert!((&err).__private_capture_as_error().to_borrowed_error().is_some());
    }
}
