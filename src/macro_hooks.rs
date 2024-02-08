use core::{any::Any, fmt, ops::ControlFlow};

use emit_core::{
    clock::{Clock, ErasedClock},
    ctxt::{Ctxt, ErasedCtxt},
    emitter::Emitter,
    empty::Empty,
    event::Event,
    extent::Extent,
    filter::Filter,
    props::Props,
    str::ToStr,
    template::Template,
    value::{ToValue, Value},
    well_known::{SPAN_ID_KEY, SPAN_PARENT_KEY, TRACE_ID_KEY},
};

use emit_core::extent::ToExtent;
#[cfg(feature = "std")]
use std::error::Error;

use crate::{
    base_emit,
    frame::Frame,
    id::{SpanId, TraceId},
    template::{Formatter, Part},
    timer::TimerGuard,
    IdRng, Level, Str, Timer,
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

impl<'a> __PrivateKeyHook for Str<'a> {
    fn __private_key_as_default(self) -> Self {
        self
    }

    fn __private_key_as_static(self, key: &'static str) -> Self {
        Str::new(key)
    }

    fn __private_key_as<K: Into<Self>>(self, key: K) -> Self {
        key.into()
    }
}

#[track_caller]
pub fn __private_format(tpl: Template, props: impl Props) -> String {
    let mut s = String::new();
    tpl.render(props).write(&mut s).expect("infallible write");

    s
}

#[track_caller]
pub fn __private_emit(
    to: impl Emitter,
    when: impl Filter,
    extent: impl ToExtent,
    tpl: Template,
    props: impl Props,
) {
    let rt = crate::runtime::shared();

    base_emit(
        rt.emitter().and(to),
        rt.filter().and(when),
        rt.ctxt(),
        extent.to_extent().or_else(|| rt.now().to_extent()),
        tpl,
        props,
    );
}

#[track_caller]
pub fn __private_push_ctxt(props: impl Props) -> Frame<&'static (dyn ErasedCtxt + Send + Sync)> {
    let rt = crate::runtime::shared();

    Frame::new_push(rt.ctxt(), props)
}

#[track_caller]
pub fn __private_root_ctxt(props: impl Props) -> Frame<&'static (dyn ErasedCtxt + Send + Sync)> {
    let rt = crate::runtime::shared();

    Frame::new_root(rt.ctxt(), props)
}

#[track_caller]
pub fn __private_push_span_ctxt(
    when: impl Filter,
    tpl: Template,
    ctxt_props: impl Props,
    evt_props: impl Props,
) -> (
    Frame<Option<&'static (dyn ErasedCtxt + Send + Sync)>>,
    Option<Timer<&'static (dyn ErasedClock + Send + Sync)>>,
) {
    struct TraceContext {
        trace_id: Option<TraceId>,
        span_parent: Option<SpanId>,
        span_id: Option<SpanId>,
    }

    impl Props for TraceContext {
        fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
            &'kv self,
            mut for_each: F,
        ) -> ControlFlow<()> {
            if let Some(ref trace_id) = self.trace_id {
                for_each(TRACE_ID_KEY.to_str(), trace_id.to_value())?;
            }

            if let Some(ref span_parent) = self.span_parent {
                for_each(SPAN_PARENT_KEY.to_str(), span_parent.to_value())?;
            }

            if let Some(ref span_id) = self.span_id {
                for_each(SPAN_ID_KEY.to_str(), span_id.to_value())?;
            }

            ControlFlow::Continue(())
        }
    }

    let rt = crate::runtime::shared();

    let mut trace_id = None;
    let mut span_parent = None;

    rt.with_current(|current| {
        trace_id = current.pull::<_, TraceId>(TRACE_ID_KEY);
        span_parent = current.pull::<_, SpanId>(SPAN_ID_KEY);
    });

    trace_id = trace_id.or_else(|| rt.gen_trace_id());
    let span_id = rt.gen_span_id();

    let trace_ctxt = TraceContext {
        trace_id,
        span_parent,
        span_id,
    };

    let timer = Timer::start(*rt.clock());

    if when.matches(&Event::new(
        timer.extent().map(|extent| *extent.as_point()),
        tpl.by_ref(),
        ctxt_props.by_ref().chain(&trace_ctxt).chain(&evt_props),
    )) {
        (
            Frame::new_push(Some(rt.ctxt()), trace_ctxt.chain(ctxt_props)),
            Some(timer),
        )
    } else {
        (Frame::new_push(None, Empty), None)
    }
}

#[track_caller]
pub fn __private_begin_span<C: Clock, F: FnOnce(Option<Extent>)>(
    timer: Option<Timer<C>>,
    default_complete: F,
) -> TimerGuard<C, F> {
    if let Some(timer) = timer {
        TimerGuard::new(timer, default_complete)
    } else {
        TimerGuard::disabled()
    }
}

pub use core::module_path as __private_module;

#[repr(transparent)]
pub struct __PrivateMacroProps<'a>([(Str<'a>, Option<Value<'a>>)]);

impl __PrivateMacroProps<'static> {
    pub fn new(props: &'static [(Str<'static>, Option<Value<'static>>)]) -> &'static Self {
        Self::new_ref(props)
    }
}

impl<'a> __PrivateMacroProps<'a> {
    pub fn new_ref<'b>(props: &'b [(Str<'a>, Option<Value<'a>>)]) -> &'b Self {
        unsafe {
            &*(props as *const [(Str<'a>, Option<Value<'a>>)] as *const __PrivateMacroProps<'a>)
        }
    }
}

impl<'a> Props for __PrivateMacroProps<'a> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
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

    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.to_str();

        self.0
            .binary_search_by(|(k, _)| k.cmp(&key))
            .ok()
            .and_then(|i| self.0[i].1.as_ref().map(|v| v.by_ref()))
    }
}
