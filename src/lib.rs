#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::{future::Future, ops::Range};

use crate::local_frame::{LocalFrame, LocalFrameFuture};
use emit_core::{
    ctxt::Ctxt,
    filter::Filter,
    id::{IdGen, SpanId, TraceId},
    target::Target,
    time::{Clock, Timer},
};

#[doc(inline)]
pub use emit_macros::*;

#[doc(inline)]
pub use emit_core::{
    ctxt, empty, event, filter, id, key, level, props, target, template, time, value, well_known,
};

pub mod local_frame;

pub use self::{
    event::Event, key::Key, level::Level, props::Props, template::Template, time::Timestamp,
    value::Value,
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

#[track_caller]
pub fn with(props: impl Props) -> LocalFrame<impl Ctxt + Send + Sync + 'static> {
    base_with(emit_core::ambient::get(), props)
}

#[track_caller]
pub fn start_timer() -> Timer<impl Clock + Send + Sync + Copy + 'static> {
    Timer::start(emit_core::ambient::get())
}

#[track_caller]
pub fn new_span_id() -> Option<SpanId> {
    emit_core::ambient::get().new_span_id()
}

#[track_caller]
pub fn new_trace_id() -> Option<TraceId> {
    emit_core::ambient::get().new_trace_id()
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
