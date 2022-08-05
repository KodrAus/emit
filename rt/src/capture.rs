use crate::{
    std::{any::Any, fmt},
    value::ValueBag,
};

#[cfg(feature = "std")]
use crate::std::error::Error;

#[cfg(feature = "serde")]
use serde_lib::Serialize;

#[cfg(feature = "sval")]
use sval_lib::value::Value;

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

/**
A marker trait used to allow a single implementation for capturing common unsized
types for any sized capture strategy.
*/
pub trait CaptureSized {}

pub enum CaptureDisplay {}
pub enum CaptureAnonDisplay {}

pub enum CaptureDebug {}
pub enum CaptureAnonDebug {}

pub enum CaptureSval {}
pub enum CaptureAnonSval {}

pub enum CaptureSerde {}
pub enum CaptureAnonSerde {}

pub enum CaptureError {}

impl CaptureSized for CaptureDisplay {}
impl CaptureSized for CaptureDebug {}
impl CaptureSized for CaptureSval {}
impl CaptureSized for CaptureSerde {}
impl CaptureSized for CaptureError {}

impl<T: CaptureSized + ?Sized> Capture<T> for str {
    fn capture(&self) -> ValueBag {
        ValueBag::from(self)
    }
}

impl<T> Capture<CaptureDisplay> for T
where
    T: fmt::Display + Any,
{
    fn capture(&self) -> ValueBag {
        ValueBag::capture_display(self)
    }
}

impl Capture<CaptureDisplay> for dyn fmt::Display {
    fn capture(&self) -> ValueBag {
        ValueBag::from_dyn_display(self)
    }
}

impl<T> Capture<CaptureAnonDisplay> for T
where
    T: fmt::Display,
{
    fn capture(&self) -> ValueBag {
        ValueBag::from_display(self)
    }
}

impl<T> Capture<CaptureDebug> for T
where
    T: fmt::Debug + Any,
{
    fn capture(&self) -> ValueBag {
        ValueBag::capture_debug(self)
    }
}

impl Capture<CaptureDebug> for dyn fmt::Debug {
    fn capture(&self) -> ValueBag {
        ValueBag::from_dyn_debug(self)
    }
}

impl<T> Capture<CaptureAnonDebug> for T
where
    T: fmt::Debug,
{
    fn capture(&self) -> ValueBag {
        ValueBag::from_debug(self)
    }
}

#[cfg(feature = "sval")]
impl<T> Capture<CaptureSval> for T
where
    T: Value + Any,
{
    fn capture(&self) -> ValueBag {
        ValueBag::capture_sval1(self)
    }
}

#[cfg(feature = "sval")]
impl Capture<CaptureSval> for dyn Value {
    fn capture(&self) -> ValueBag {
        ValueBag::from_dyn_sval1(self)
    }
}

#[cfg(feature = "sval")]
impl<T> Capture<CaptureAnonSval> for T
where
    T: Value,
{
    fn capture(&self) -> ValueBag {
        ValueBag::from_sval1(self)
    }
}

#[cfg(feature = "serde")]
impl<T> Capture<CaptureSerde> for T
where
    T: Serialize + Any,
{
    fn capture(&self) -> ValueBag {
        ValueBag::capture_serde1(self)
    }
}

#[cfg(feature = "serde")]
impl<T> Capture<CaptureAnonSerde> for T
where
    T: Serialize,
{
    fn capture(&self) -> ValueBag {
        ValueBag::from_serde1(self)
    }
}

#[cfg(feature = "std")]
impl<T> Capture<CaptureError> for T
where
    T: Error + 'static,
{
    fn capture(&self) -> ValueBag {
        ValueBag::capture_error(self)
    }
}

#[cfg(feature = "std")]
impl<'a> Capture<CaptureError> for (dyn Error + 'static) {
    fn capture(&self) -> ValueBag {
        ValueBag::from_dyn_error(self)
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
- It uses clumsily uglified names that are unlikely to clash in non-hygienic
contexts. (We're expecting non-hygienic spans to support value interpolation).
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

    fn __private_capture_anon_as_display(&self) -> ValueBag
    where
        Self: Capture<CaptureAnonDisplay>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_debug(&self) -> ValueBag
    where
        Self: Capture<CaptureDebug>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_debug(&self) -> ValueBag
    where
        Self: Capture<CaptureAnonDebug>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_sval(&self) -> ValueBag
    where
        Self: Capture<CaptureSval>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_sval(&self) -> ValueBag
    where
        Self: Capture<CaptureAnonSval>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_serde(&self) -> ValueBag
    where
        Self: Capture<CaptureSerde>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_serde(&self) -> ValueBag
    where
        Self: Capture<CaptureAnonSerde>,
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

impl<T: ?Sized> __PrivateCapture for T {}

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
        let _ = SomeType.__private_capture_as_default();

        // Capture a structured number
        assert_eq!(Some(42u64), 42u64.__private_capture_as_default().to_u64());

        // Capture a borrowed (non-static) string
        let v: &str = &String::from("a string");
        assert_eq!(
            Some("a string"),
            v.__private_capture_as_default().to_borrowed_str()
        );

        // Capture a value with parens
        let _ = (SomeType).__private_capture_as_default();

        // Capture and borrow a string as an expression
        let v = SomeType;
        match (
            v.__private_capture_as_default(),
            String::from("a string").__private_capture_as_default(),
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
        let _ = SomeType.__private_capture_as_display();
        let _ = (&&&SomeType).__private_capture_anon_as_display();

        // Capture a `&dyn Display`
        let v: &dyn fmt::Display = &SomeType;
        let _ = v.__private_capture_as_display();
        let _ = (&&&v).__private_capture_anon_as_display();
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
        let _ = SomeType.__private_capture_as_debug();
        let _ = (&&&SomeType).__private_capture_anon_as_debug();

        // Capture a `&dyn Debug`
        let v: &dyn fmt::Debug = &SomeType;
        let _ = v.__private_capture_as_debug();
        let _ = (&&&v).__private_capture_anon_as_debug();
    }

    #[test]
    #[cfg(feature = "sval")]
    fn capture_sval() {
        use sval_lib::value::{self, Value};

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
        let _ = map.__private_capture_as_sval();
        let _ = (&&&map).__private_capture_anon_as_sval();

        // Capture a `&dyn Value`
        let v: &dyn Value = &map;
        let _ = v.__private_capture_as_sval();
        let _ = (&&&v).__private_capture_anon_as_sval();
    }

    #[test]
    #[cfg(feature = "serde")]
    fn capture_serde() {
        let tuple = (1, 2, 3, 4, 5);

        let _ = tuple.__private_capture_as_serde();
        let _ = (&&&tuple).__private_capture_anon_as_serde();
    }

    #[test]
    #[cfg(feature = "std")]
    fn capture_error() {
        use crate::std::io;

        // Capture an arbitrary `Error`
        let err = io::Error::from(io::ErrorKind::Other);
        assert!(err
            .__private_capture_as_error()
            .to_borrowed_error()
            .is_some());

        // Capture a `&dyn Error`
        let err: &dyn Error = &err;
        assert!(err
            .__private_capture_as_error()
            .to_borrowed_error()
            .is_some());
    }
}
