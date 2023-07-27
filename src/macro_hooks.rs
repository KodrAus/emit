use core::{any::Any, fmt, future::Future};

use emit_core::{
    ambient,
    ctxt::Ctxt,
    filter::Filter,
    level::Level,
    props::Props,
    target::Target,
    template::Template,
    time::{Clock, Extent},
    value::Value,
    well_known::LEVEL_KEY,
};

#[cfg(feature = "std")]
use std::error::Error;

use crate::{
    base_emit, base_with, base_with_future,
    ctxt::{LocalFrame, LocalFrameFuture},
    template::{Formatter, Part},
    Key,
};

/**
Similar to `log`'s `ToValue` trait, but with a generic pivot parameter.

The parameter lets us choose the way values are captured, since many
types can be captured in lots of different ways.
*/
pub trait Capture<T: ?Sized> {
    fn capture(&self) -> Value;
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
    fn capture(&self) -> Value {
        Value::from(self)
    }
}

impl<T> Capture<CaptureDisplay> for T
where
    T: fmt::Display + Any,
{
    fn capture(&self) -> Value {
        value_bag::ValueBag::capture_display(self).into()
    }
}

impl Capture<CaptureDisplay> for dyn fmt::Display {
    fn capture(&self) -> Value {
        value_bag::ValueBag::from_dyn_display(self).into()
    }
}

impl<T> Capture<CaptureAnonDisplay> for T
where
    T: fmt::Display,
{
    fn capture(&self) -> Value {
        value_bag::ValueBag::from_display(self).into()
    }
}

impl<T> Capture<CaptureDebug> for T
where
    T: fmt::Debug + Any,
{
    fn capture(&self) -> Value {
        value_bag::ValueBag::capture_debug(self).into()
    }
}

impl Capture<CaptureDebug> for dyn fmt::Debug {
    fn capture(&self) -> Value {
        value_bag::ValueBag::from_dyn_debug(self).into()
    }
}

impl<T> Capture<CaptureAnonDebug> for T
where
    T: fmt::Debug,
{
    fn capture(&self) -> Value {
        value_bag::ValueBag::from_debug(self).into()
    }
}

#[cfg(feature = "sval")]
impl<T> Capture<CaptureSval> for T
where
    T: sval::Value + Any,
{
    fn capture(&self) -> Value {
        value_bag::ValueBag::capture_sval2(self).into()
    }
}

#[cfg(feature = "sval")]
impl<T> Capture<CaptureAnonSval> for T
where
    T: sval::Value,
{
    fn capture(&self) -> Value {
        value_bag::ValueBag::from_sval2(self).into()
    }
}

#[cfg(feature = "serde")]
impl<T> Capture<CaptureSerde> for T
where
    T: serde::Serialize + Any,
{
    fn capture(&self) -> Value {
        value_bag::ValueBag::capture_serde1(self).into()
    }
}

#[cfg(feature = "serde")]
impl<T> Capture<CaptureAnonSerde> for T
where
    T: serde::Serialize,
{
    fn capture(&self) -> Value {
        value_bag::ValueBag::from_serde1(self).into()
    }
}

#[cfg(feature = "std")]
impl<T> Capture<CaptureError> for T
where
    T: Error + 'static,
{
    fn capture(&self) -> Value {
        value_bag::ValueBag::capture_error(self).into()
    }
}

#[cfg(feature = "std")]
impl<'a> Capture<CaptureError> for (dyn Error + 'static) {
    fn capture(&self) -> Value {
        value_bag::ValueBag::from_dyn_error(self).into()
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
pub trait __PrivateCaptureHook {
    fn __private_capture_as_default(&self) -> Value
    where
        Self: Capture<WithDefault>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_display(&self) -> Value
    where
        Self: Capture<CaptureDisplay>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_display(&self) -> Value
    where
        Self: Capture<CaptureAnonDisplay>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_debug(&self) -> Value
    where
        Self: Capture<CaptureDebug>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_debug(&self) -> Value
    where
        Self: Capture<CaptureAnonDebug>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_sval(&self) -> Value
    where
        Self: Capture<CaptureSval>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_sval(&self) -> Value
    where
        Self: Capture<CaptureAnonSval>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_serde(&self) -> Value
    where
        Self: Capture<CaptureSerde>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_serde(&self) -> Value
    where
        Self: Capture<CaptureAnonSerde>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_error(&self) -> Value
    where
        Self: Capture<CaptureError>,
    {
        Capture::capture(self)
    }
}

impl<T: ?Sized> __PrivateCaptureHook for T {}

pub trait __PrivateFmtHook<'a> {
    fn __private_fmt_as_default(self) -> Self;
    fn __private_fmt_as(self, formatter: Formatter) -> Self;
}

impl<'a> __PrivateFmtHook<'a> for Part<'a> {
    fn __private_fmt_as_default(self) -> Self {
        self
    }

    fn __private_fmt_as(self, formatter: Formatter) -> Self {
        self.with_formatter(formatter)
    }
}

pub trait __PrivateKeyHook {
    fn __private_key_as_default(self) -> Self;
    fn __private_key_as_static(self, key: &'static str) -> Self;
    fn __private_key_as<K: Into<Self>>(self, key: K) -> Self
    where
        Self: Sized;
}

impl<'a> __PrivateKeyHook for Key<'a> {
    fn __private_key_as_default(self) -> Self {
        self
    }

    fn __private_key_as_static(self, key: &'static str) -> Self {
        Key::new(key)
    }

    fn __private_key_as<K: Into<Self>>(self, key: K) -> Self {
        key.into()
    }
}

#[track_caller]
pub fn __emit(
    to: impl Target,
    when: impl Filter,
    ts: Option<Extent>,
    lvl: Level,
    tpl: Template,
    props: impl Props,
) {
    let ambient = ambient::get();

    base_emit(
        to.and(ambient),
        when.and(ambient),
        ambient,
        ts.or_else(|| ambient.now().map(Extent::point)),
        tpl,
        (LEVEL_KEY, lvl).chain(props),
    );
}

#[track_caller]
pub fn __with(props: impl Props) -> LocalFrame<impl Ctxt> {
    let ambient = ambient::get();

    base_with(ambient, props)
}

#[track_caller]
pub fn __with_future<F: Future>(
    props: impl Props,
    future: F,
) -> LocalFrameFuture<impl Ctxt + Send + Sync + 'static, F> {
    let ambient = ambient::get();

    base_with_future(ambient, props, future)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fmt, string::String};

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
        let tuple = (1, 2, 3, 4, 5);

        // Capture an arbitrary `Value`
        let _ = tuple.__private_capture_as_sval();
        let _ = (&&&tuple).__private_capture_anon_as_sval();
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
        use std::io;

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
