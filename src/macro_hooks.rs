use core::{any::Any, fmt, ops::ControlFlow};

use emit_core::{
    clock::Clock,
    ctxt::Ctxt,
    emitter::Emitter,
    extent::{Extent, ToExtent},
    filter::Filter,
    path::Path,
    props::Props,
    rng::Rng,
    runtime::Runtime,
    str::{Str, ToStr},
    template::{Formatter, Part, Template},
    value::{ToValue, Value},
};

#[cfg(feature = "alloc")]
use emit_core::{
    empty::Empty,
    event::Event,
    well_known::{KEY_SPAN_ID, KEY_SPAN_PARENT, KEY_TRACE_ID},
};

#[cfg(feature = "alloc")]
use crate::frame::Frame;

#[cfg(feature = "std")]
use std::error::Error;

use crate::{
    base_emit,
    timer::TimerGuard,
    trace::{SpanId, TraceId},
    Level, Timer,
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

pub enum CaptureValue {}
pub enum CaptureAnonValue {}

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
impl CaptureStr for CaptureValue {}

impl<T: CaptureStr + ?Sized> Capture<T> for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
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
        Some(Value::capture_display(self))
    }
}

impl Capture<CaptureDisplay> for dyn fmt::Display {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T> Capture<CaptureAnonDisplay> for T
where
    T: fmt::Display,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::from_display(self))
    }
}

impl<T> Capture<CaptureDebug> for T
where
    T: fmt::Debug + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_debug(self))
    }
}

impl Capture<CaptureDebug> for dyn fmt::Debug {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T> Capture<CaptureAnonDebug> for T
where
    T: fmt::Debug,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::from_debug(self))
    }
}

impl<T> Capture<CaptureValue> for T
where
    T: ToValue + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T> Capture<CaptureAnonValue> for T
where
    T: ToValue,
{
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[cfg(feature = "sval")]
impl<T> Capture<CaptureSval> for T
where
    T: sval::Value + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_sval(self))
    }
}

#[cfg(feature = "sval")]
impl<T> Capture<CaptureAnonSval> for T
where
    T: sval::Value,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::from_sval(self))
    }
}

#[cfg(feature = "serde")]
impl<T> Capture<CaptureSerde> for T
where
    T: serde::Serialize + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_serde(self))
    }
}

#[cfg(feature = "serde")]
impl<T> Capture<CaptureAnonSerde> for T
where
    T: serde::Serialize,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::from_serde(self))
    }
}

#[cfg(feature = "std")]
impl<T> Capture<CaptureError> for T
where
    T: Error + 'static,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_error(self))
    }
}

#[cfg(feature = "std")]
impl<'a> Capture<CaptureError> for (dyn Error + 'static) {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
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

    fn __private_capture_as_value(&self) -> Option<Value>
    where
        Self: Capture<CaptureValue>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_value(&self) -> Option<Value>
    where
        Self: Capture<CaptureAnonValue>,
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
#[cfg(feature = "alloc")]
pub fn __private_format(tpl: Template, props: impl Props) -> alloc::string::String {
    let mut s = alloc::string::String::new();
    tpl.render(props).write(&mut s).expect("infallible write");

    s
}

struct FirstDefined<A, B>(Option<A>, B);

impl<A: Filter, B: Filter> Filter for FirstDefined<A, B> {
    fn matches<P: Props>(&self, evt: &emit_core::event::Event<P>) -> bool {
        if let Some(ref first) = self.0 {
            return first.matches(evt);
        }

        self.1.matches(evt)
    }
}

#[track_caller]
pub fn __private_filter_span_complete() -> Option<impl Filter + Send + Sync + 'static> {
    Some(crate::filter::always())
}

#[track_caller]
pub fn __private_emit<'a, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng>(
    rt: &'a Runtime<E, F, C, T, R>,
    source: impl Into<Path<'a>>,
    when: Option<impl Filter>,
    extent: impl ToExtent,
    tpl: Template,
    props: impl Props,
) {
    base_emit(
        rt.emitter(),
        source.into(),
        FirstDefined(when, rt.filter()),
        rt.ctxt(),
        extent.to_extent().or_else(|| rt.now().to_extent()),
        tpl,
        props,
    );
}

#[track_caller]
#[cfg(feature = "alloc")]
pub fn __private_push_span_ctxt<'a, 'b, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng>(
    rt: &'a Runtime<E, F, C, T, R>,
    source: impl Into<Path<'b>>,
    when: Option<impl Filter>,
    tpl: Template<'b>,
    ctxt_props: impl Props,
    evt_props: impl Props,
) -> (Frame<Option<&'a C>>, Option<Timer<&'a T>>) {
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
                for_each(KEY_TRACE_ID.to_str(), trace_id.to_value())?;
            }

            if let Some(ref span_parent) = self.span_parent {
                for_each(KEY_SPAN_PARENT.to_str(), span_parent.to_value())?;
            }

            if let Some(ref span_id) = self.span_id {
                for_each(KEY_SPAN_ID.to_str(), span_id.to_value())?;
            }

            ControlFlow::Continue(())
        }
    }

    let (mut trace_id, span_parent) = rt.with_current(|current| {
        (
            current.pull::<TraceId, _>(KEY_TRACE_ID),
            current.pull::<SpanId, _>(KEY_SPAN_ID),
        )
    });

    trace_id = trace_id.or_else(|| TraceId::random(rt));
    let span_id = SpanId::random(rt);

    let trace_ctxt = TraceContext {
        trace_id,
        span_parent,
        span_id,
    };

    let timer = Timer::start(rt.clock());

    if FirstDefined(when, rt.filter()).matches(&Event::new(
        source,
        timer.extent().map(|extent| *extent.as_point()),
        tpl,
        ctxt_props.by_ref().chain(&trace_ctxt).chain(&evt_props),
    )) {
        (
            Frame::push(Some(rt.ctxt()), trace_ctxt.chain(ctxt_props)),
            Some(timer),
        )
    } else {
        (Frame::push(None, Empty), None)
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

    fn count(&self) -> usize {
        self.0.len()
    }

    fn is_sorted(&self) -> bool {
        true
    }

    fn is_unique(&self) -> bool {
        true
    }
}
