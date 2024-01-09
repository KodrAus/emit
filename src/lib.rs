#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use emit_core::extent::ToExtent;

use crate::frame::Frame;

#[doc(inline)]
pub use emit_macros::*;

#[doc(inline)]
pub use emit_core::{
    clock, ctxt, emitter, empty, event, extent, filter, id, key, level, props, template, timestamp,
    value, well_known,
};

pub mod frame;

pub use self::{
    clock::{Clock, Timer},
    ctxt::Ctxt,
    emitter::Emitter,
    event::Event,
    extent::Extent,
    filter::Filter,
    id::{IdGen, SpanId, TraceId},
    key::Key,
    level::Level,
    props::Props,
    template::Template,
    timestamp::Timestamp,
    value::Value,
    well_known::WellKnown,
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
    let ambient = emit_core::ambient::get();

    let tpl = evt.tpl();
    let props = evt.props();
    let extent = evt.extent().cloned().or_else(|| ambient.now().to_extent());

    base_emit(ambient, ambient, ambient, extent, tpl, props);
}

pub type PushCtxt = Frame<emit_core::ambient::Get>;

pub type StartTimer = Timer<emit_core::ambient::Get>;

#[track_caller]
pub fn now() -> Option<Timestamp> {
    emit_core::ambient::get().now()
}

#[track_caller]
pub fn push_ctxt(props: impl Props) -> PushCtxt {
    base_push_ctxt(emit_core::ambient::get(), props)
}

#[track_caller]
pub fn current_ctxt() -> PushCtxt {
    base_push_ctxt(emit_core::ambient::get(), empty::Empty)
}

#[track_caller]
pub fn start_timer() -> StartTimer {
    Timer::start(emit_core::ambient::get())
}

#[track_caller]
pub fn new_span_id() -> Option<SpanId> {
    emit_core::ambient::get().new_span_id()
}

#[track_caller]
pub fn current_span_id() -> Option<SpanId> {
    let mut span_id = None;

    emit_core::ambient::get().with_current(|ctxt| {
        span_id = ctxt.span_id();
    });

    span_id
}

#[track_caller]
pub fn new_trace_id() -> Option<TraceId> {
    emit_core::ambient::get().new_trace_id()
}

#[track_caller]
pub fn current_trace_id() -> Option<TraceId> {
    let mut trace_id = None;

    emit_core::ambient::get().with_current(|ctxt| {
        trace_id = ctxt.trace_id();
    });

    trace_id
}

#[doc(hidden)]
pub mod __private {
    pub use crate::macro_hooks::*;
    pub use core;
}
