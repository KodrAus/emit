#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::future::Future;

use emit_core::extent::ToExtent;

use crate::local_frame::{LocalFrame, LocalFrameFuture};

#[doc(inline)]
pub use emit_macros::*;

#[doc(inline)]
pub use emit_core::{
    clock, ctxt, empty, event, extent, filter, id, key, level, props, target, template, timestamp,
    value, well_known,
};

pub mod local_frame;

pub use self::{
    clock::{Clock, Timer},
    ctxt::Ctxt,
    event::Event,
    extent::Extent,
    filter::Filter,
    id::{IdGen, SpanId, TraceId},
    key::Key,
    level::Level,
    props::Props,
    target::Target,
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
    to: impl Target,
    when: impl Filter,
    ctxt: impl Ctxt,
    ts: impl ToExtent,
    tpl: Template,
    props: impl Props,
) {
    ctxt.with_current(|ctxt| {
        let evt = Event::new(ts, tpl, props.chain(ctxt));

        if when.matches(&evt) {
            to.event(&evt);
        }
    });
}

#[track_caller]
fn base_with<C: Ctxt>(ctxt: C, props: impl Props) -> LocalFrame<C> {
    LocalFrame::new(ctxt, props)
}

#[track_caller]
fn base_with_future<C: Ctxt, F: Future>(
    ctxt: C,
    props: impl Props,
    future: F,
) -> LocalFrameFuture<C, F> {
    LocalFrameFuture::new(ctxt, props, future)
}

#[track_caller]
pub fn emit(evt: &Event<impl Props>) {
    let ambient = emit_core::ambient::get();

    let tpl = evt.tpl();
    let props = evt.props();

    base_emit(
        ambient,
        ambient,
        ambient,
        evt.extent().or_else(|| ambient.now()),
        tpl,
        props,
    );
}

pub type With = LocalFrame<emit_core::ambient::Get>;

pub type StartTimer = Timer<emit_core::ambient::Get>;

#[track_caller]
pub fn with(props: impl Props) -> With {
    base_with(emit_core::ambient::get(), props)
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
