use core::{any::Any, fmt, ops::ControlFlow};

use emit_core::{
    ambient,
    clock::Clock,
    emitter::Emitter,
    filter::Filter,
    id::{SpanId, TraceId},
    key::ToKey,
    level::Level,
    props::Props,
    template::Template,
    value::{ToValue, Value},
};

use emit_core::extent::ToExtent;
#[cfg(feature = "std")]
use std::error::Error;

use crate::{
    base_emit, base_push_ctxt,
    frame::Frame,
    template::{Formatter, Part},
    Key,
};

/**
Similar to `log`'s `ToValue` trait, but with a generic pivot parameter.

The parameter lets us choose the way values are captured, since many
types can be captured in lots of different ways.
*/
pub trait Capture<T: ?Sized> {
    fn capture(&self) -> Option<Value>;
}

/**
The default capturing method is with the `Value::capture_display` method.
*/
pub type WithDefault = CaptureDisplay;

/**
A marker trait used to allow a single implementation for capturing common unsized
types for any sized capture strategy.
*/
pub trait CaptureStr {}

pub enum CaptureDisplay {}
pub enum CaptureAnonDisplay {}

pub enum CaptureDebug {}
pub enum CaptureAnonDebug {}

pub enum CaptureSval {}
pub enum CaptureAnonSval {}

pub enum CaptureSerde {}
pub enum CaptureAnonSerde {}

pub enum CaptureLevel {}

pub enum CaptureError {}

pub enum CaptureSpanId {}
pub enum CaptureTraceId {}

impl CaptureStr for CaptureDisplay {}
impl CaptureStr for CaptureDebug {}
impl CaptureStr for CaptureSval {}
impl CaptureStr for CaptureSerde {}
impl CaptureStr for CaptureLevel {}
impl CaptureStr for CaptureError {}
impl CaptureStr for CaptureSpanId {}
impl CaptureStr for CaptureTraceId {}

impl<T: CaptureStr + ?Sized> Capture<T> for str {
    fn capture(&self) -> Option<Value> {
        Some(Value::from(self))
    }
}

impl Capture<CaptureSpanId> for SpanId {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T: Capture<CaptureSpanId>> Capture<CaptureSpanId> for Option<T> {
    fn capture(&self) -> Option<Value> {
        self.as_ref().and_then(|v| v.capture())
    }
}

impl Capture<CaptureTraceId> for TraceId {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T: Capture<CaptureTraceId>> Capture<CaptureTraceId> for Option<T> {
    fn capture(&self) -> Option<Value> {
        self.as_ref().and_then(|v| v.capture())
    }
}

impl Capture<CaptureLevel> for Level {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T: Capture<CaptureLevel>> Capture<CaptureLevel> for Option<T> {
    fn capture(&self) -> Option<Value> {
        self.as_ref().and_then(|v| v.capture())
    }
}

impl<T> Capture<CaptureDisplay> for T
where
    T: fmt::Display + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(value_bag::ValueBag::capture_display(self).into())
    }
}

impl Capture<CaptureDisplay> for dyn fmt::Display {
    fn capture(&self) -> Option<Value> {
        Some(value_bag::ValueBag::from_dyn_display(self).into())
    }
}

impl<T> Capture<CaptureAnonDisplay> for T
where
    T: fmt::Display,
{
    fn capture(&self) -> Option<Value> {
        Some(value_bag::ValueBag::from_display(self).into())
    }
}

impl<T> Capture<CaptureDebug> for T
where
    T: fmt::Debug + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(value_bag::ValueBag::capture_debug(self).into())
    }
}

impl Capture<CaptureDebug> for dyn fmt::Debug {
    fn capture(&self) -> Option<Value> {
        Some(value_bag::ValueBag::from_dyn_debug(self).into())
    }
}

impl<T> Capture<CaptureAnonDebug> for T
where
    T: fmt::Debug,
{
    fn capture(&self) -> Option<Value> {
        Some(value_bag::ValueBag::from_debug(self).into())
    }
}

#[cfg(feature = "sval")]
impl<T> Capture<CaptureSval> for T
where
    T: sval::Value + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(value_bag::ValueBag::capture_sval2(self).into())
    }
}

#[cfg(feature = "sval")]
impl<T> Capture<CaptureAnonSval> for T
where
    T: sval::Value,
{
    fn capture(&self) -> Option<Value> {
        Some(value_bag::ValueBag::from_sval2(self).into())
    }
}

#[cfg(feature = "serde")]
impl<T> Capture<CaptureSerde> for T
where
    T: serde::Serialize + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(value_bag::ValueBag::capture_serde1(self).into())
    }
}

#[cfg(feature = "serde")]
impl<T> Capture<CaptureAnonSerde> for T
where
    T: serde::Serialize,
{
    fn capture(&self) -> Option<Value> {
        Some(value_bag::ValueBag::from_serde1(self).into())
    }
}

#[cfg(feature = "std")]
impl<T> Capture<CaptureError> for T
where
    T: Error + 'static,
{
    fn capture(&self) -> Option<Value> {
        Some(value_bag::ValueBag::capture_error(self).into())
    }
}

#[cfg(feature = "std")]
impl<'a> Capture<CaptureError> for (dyn Error + 'static) {
    fn capture(&self) -> Option<Value> {
        Some(value_bag::ValueBag::from_dyn_error(self).into())
    }
}

pub trait __PrivateOptionalCaptureHook {
    fn __private_optional_capture_some(&self) -> Option<&Self>;

    fn __private_optional_capture_option(&self) -> &Self;
}

impl<T: ?Sized> __PrivateOptionalCaptureHook for T {
    fn __private_optional_capture_some(&self) -> Option<&Self> {
        Some(self)
    }

    fn __private_optional_capture_option(&self) -> &Self {
        self
    }
}

pub trait __PrivateOptionalMapHook<T> {
    fn __private_optional_map_some<F: FnOnce(T) -> Option<U>, U>(self, map: F) -> Option<U>;

    fn __private_optional_map_option<'a, F: FnOnce(&'a T) -> Option<U>, U: 'a>(
        &'a self,
        map: F,
    ) -> Option<U>
    where
        T: 'a;
}

impl<T> __PrivateOptionalMapHook<T> for Option<T> {
    fn __private_optional_map_some<F: FnOnce(T) -> Option<U>, U>(self, map: F) -> Option<U> {
        self.and_then(map)
    }

    fn __private_optional_map_option<'a, F: FnOnce(&'a T) -> Option<U>, U: 'a>(
        &'a self,
        map: F,
    ) -> Option<U> {
        self.as_ref().and_then(map)
    }
}

pub trait __PrivateInterpolatedHook {
    fn __private_interpolated(self) -> Self;
    fn __private_uninterpolated(self) -> Self;

    fn __private_captured(self) -> Self;
    fn __private_uncaptured(self) -> Self;
}

impl<T> __PrivateInterpolatedHook for T {
    fn __private_interpolated(self) -> Self {
        self
    }

    fn __private_uninterpolated(self) -> Self {
        self
    }

    fn __private_captured(self) -> Self {
        self
    }

    fn __private_uncaptured(self) -> Self {
        self
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
    fn __private_capture_as_default(&self) -> Option<Value>
    where
        Self: Capture<WithDefault>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_display(&self) -> Option<Value>
    where
        Self: Capture<CaptureDisplay>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_display(&self) -> Option<Value>
    where
        Self: Capture<CaptureAnonDisplay>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_debug(&self) -> Option<Value>
    where
        Self: Capture<CaptureDebug>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_debug(&self) -> Option<Value>
    where
        Self: Capture<CaptureAnonDebug>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_sval(&self) -> Option<Value>
    where
        Self: Capture<CaptureSval>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_sval(&self) -> Option<Value>
    where
        Self: Capture<CaptureAnonSval>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_serde(&self) -> Option<Value>
    where
        Self: Capture<CaptureSerde>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_serde(&self) -> Option<Value>
    where
        Self: Capture<CaptureAnonSerde>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_level(&self) -> Option<Value>
    where
        Self: Capture<CaptureLevel>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_error(&self) -> Option<Value>
    where
        Self: Capture<CaptureError>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_span_id(&self) -> Option<Value>
    where
        Self: Capture<CaptureSpanId>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_trace_id(&self) -> Option<Value>
    where
        Self: Capture<CaptureTraceId>,
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
pub fn __private_emit(
    to: impl Emitter,
    when: impl Filter,
    extent: impl ToExtent,
    tpl: Template,
    props: impl Props,
) {
    let ambient = ambient::get();

    base_emit(
        to.and(ambient),
        when.and(ambient),
        ambient,
        extent.to_extent().or_else(|| ambient.now().to_extent()),
        tpl,
        props,
    );
}

#[track_caller]
pub fn __private_push_ctxt(props: impl Props) -> Frame<emit_core::ambient::Get> {
    let ambient = ambient::get();

    base_push_ctxt(ambient, props)
}

pub use core::module_path as loc;

#[repr(transparent)]
pub struct __PrivateMacroProps<'a>([(Key<'a>, Option<Value<'a>>)]);

impl __PrivateMacroProps<'static> {
    pub fn new(props: &'static [(Key<'static>, Option<Value<'static>>)]) -> &'static Self {
        Self::new_ref(props)
    }
}

impl<'a> __PrivateMacroProps<'a> {
    pub fn new_ref<'b>(props: &'b [(Key<'a>, Option<Value<'a>>)]) -> &'b Self {
        unsafe {
            &*(props as *const [(Key<'a>, Option<Value<'a>>)] as *const __PrivateMacroProps<'a>)
        }
    }
}

impl<'a> Props for __PrivateMacroProps<'a> {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for kv in &self.0 {
            let k = &kv.0;

            if let Some(ref v) = kv.1 {
                for_each(k.by_ref(), v.by_ref())?;
            }
        }

        ControlFlow::Continue(())
    }

    fn get<'v, K: ToKey>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.to_key();

        self.0
            .binary_search_by(|(k, _)| k.cmp(&key))
            .ok()
            .and_then(|i| self.0[i].1.as_ref().map(|v| v.by_ref()))
    }
}
