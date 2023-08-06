#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::{future::Future, ops::Range};

use crate::local_frame::{LocalFrame, LocalFrameFuture};

#[doc(inline)]
pub use emit_macros::*;

#[doc(inline)]
pub use emit_core::{
    ctxt, empty, event, filter, id, key, level, props, target, template, time, value, well_known,
};

pub mod local_frame;

pub use self::{
    ctxt::Ctxt,
    event::Event,
    filter::Filter,
    id::{IdGen, SpanId, TraceId},
    key::Key,
    level::Level,
    props::Props,
    target::Target,
    template::Template,
    time::{Clock, Timer, Timestamp},
    value::Value,
};
use self::{
    ctxt::ErasedCtxt, filter::ErasedFilter, id::ErasedIdGen, target::ErasedTarget,
    time::ErasedClock,
};

mod macro_hooks;
mod platform;

#[cfg(feature = "std")]
mod setup;

#[track_caller]
fn base_emit(
    to: impl Target,
    when: impl Filter,
    ctxt: impl Ctxt,
    ts: Option<Range<Timestamp>>,
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
        evt.extent()
            .cloned()
            .or_else(|| ambient.now().map(|ts| ts..ts)),
        tpl,
        props,
    );
}

pub type Ambient = Option<
    emit_core::ambient::Ambient<
        &'static dyn ErasedTarget,
        &'static dyn ErasedFilter,
        &'static dyn ErasedCtxt,
        &'static dyn ErasedClock,
        &'static dyn ErasedIdGen,
    >,
>;

pub type With = LocalFrame<Ambient>;

pub type StartTimer = Timer<Ambient>;

#[track_caller]
pub fn ambient() -> Ambient {
    emit_core::ambient::get()
}

#[track_caller]
pub fn with(props: impl Props) -> With {
    base_with(ambient(), props)
}

#[track_caller]
pub fn start_timer() -> StartTimer {
    Timer::start(ambient())
}

#[track_caller]
pub fn new_span_id() -> Option<SpanId> {
    ambient().new_span_id()
}

#[track_caller]
pub fn new_trace_id() -> Option<TraceId> {
    ambient().new_trace_id()
}

#[cfg(feature = "std")]
pub fn setup() -> setup::Setup {
    setup::Setup::default()
}

#[doc(hidden)]
pub mod __private {
    pub use crate::macro_hooks::*;
    pub use core;
}
