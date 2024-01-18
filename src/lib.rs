#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use emit_core::extent::ToExtent;

use crate::frame::Frame;

#[doc(inline)]
pub use emit_macros::*;

#[doc(inline)]
pub use emit_core::{
    clock, ctxt, emitter, empty, event, extent, filter, props, rng, runtime, str, template,
    timestamp, value, well_known,
};

pub mod frame;
pub mod id;
pub mod level;

pub use self::{
    clock::{Clock, Timer},
    ctxt::Ctxt,
    emitter::Emitter,
    event::Event,
    extent::Extent,
    filter::Filter,
    id::{IdRng, SpanId, TraceId},
    level::Level,
    props::Props,
    rng::Rng,
    str::Str,
    template::Template,
    timestamp::Timestamp,
    value::Value,
};

mod macro_hooks;
mod platform;

#[cfg(feature = "std")]
mod setup;
#[cfg(feature = "std")]
pub use setup::*;

#[track_caller]
fn base_emit(
    to: impl Emitter,
    when: impl Filter,
    ctxt: impl Ctxt,
    ts: impl ToExtent,
    tpl: Template,
    props: impl Props,
) {
    ctxt.with_current(|ctxt| {
        let evt = Event::new(ts, tpl, props.chain(ctxt));

        if when.matches(&evt) {
            to.emit(&evt);
        }
    });
}

#[track_caller]
fn base_push_ctxt<C: Ctxt>(ctxt: C, props: impl Props) -> Frame<C> {
    Frame::new(ctxt, props)
}

#[track_caller]
pub fn emit(evt: &Event<impl Props>) {
    let ambient = emit_core::runtime::shared();

    let tpl = evt.tpl();
    let props = evt.props();
    let extent = evt.extent().cloned().or_else(|| ambient.now().to_extent());

    base_emit(ambient, ambient, ambient, extent, tpl, props);
}

pub type PushCtxt = Frame<&'static emit_core::runtime::AmbientRuntime<'static>>;

pub type StartTimer = Timer<&'static emit_core::runtime::AmbientRuntime<'static>>;

#[track_caller]
pub fn now() -> Option<Timestamp> {
    emit_core::runtime::shared().now()
}

#[track_caller]
pub fn push_ctxt(props: impl Props) -> PushCtxt {
    base_push_ctxt(emit_core::runtime::shared(), props)
}

#[track_caller]
pub fn current_ctxt() -> PushCtxt {
    base_push_ctxt(emit_core::runtime::shared(), empty::Empty)
}

#[track_caller]
pub fn start_timer() -> StartTimer {
    Timer::start(emit_core::runtime::shared())
}

#[track_caller]
pub fn new_span_id() -> Option<SpanId> {
    emit_core::runtime::shared().gen_span_id()
}

#[track_caller]
pub fn current_span_id() -> Option<SpanId> {
    let mut span_id = None;

    emit_core::runtime::shared().with_current(|ctxt| {
        span_id = ctxt.pull();
    });

    span_id
}

#[track_caller]
pub fn new_trace_id() -> Option<TraceId> {
    emit_core::runtime::shared().gen_trace_id()
}

#[track_caller]
pub fn current_trace_id() -> Option<TraceId> {
    let mut trace_id = None;

    emit_core::runtime::shared().with_current(|ctxt| {
        trace_id = ctxt.pull();
    });

    trace_id
}

#[doc(hidden)]
pub mod __private {
    pub use crate::macro_hooks::*;
    pub use core;
}
