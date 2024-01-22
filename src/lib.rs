#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use emit_core::{extent::ToExtent, well_known::LVL_KEY};

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
pub mod timer;

pub use self::{
    clock::Clock,
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
    timer::Timer,
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
pub fn now() -> Option<Timestamp> {
    emit_core::runtime::shared().now()
}

#[track_caller]
pub fn push_ctxt(props: impl Props) -> PushCtxt {
    emit_core::runtime::shared().push_ctxt(props)
}

#[track_caller]
pub fn current_ctxt() -> PushCtxt {
    emit_core::runtime::shared().current_ctxt()
}

#[track_caller]
pub fn start_timer() -> StartTimer {
    emit_core::runtime::shared().start_timer()
}

#[track_caller]
pub fn gen_span_id() -> Option<SpanId> {
    emit_core::runtime::shared().gen_span_id()
}

#[track_caller]
pub fn current_span_id() -> Option<SpanId> {
    emit_core::runtime::shared().current_span_id()
}

#[track_caller]
pub fn gen_trace_id() -> Option<TraceId> {
    emit_core::runtime::shared().gen_trace_id()
}

#[track_caller]
pub fn current_trace_id() -> Option<TraceId> {
    emit_core::runtime::shared().current_trace_id()
}

pub type PushCtxt = Frame<&'static emit_core::runtime::AmbientRuntime<'static>>;

pub type StartTimer = Timer<&'static emit_core::runtime::AmbientRuntime<'static>>;

pub trait Emit: Emitter + Filter + Ctxt + Clock + Rng {
    fn debug<P: Props>(&self, tpl: Template, props: P) {
        base_emit(
            self,
            self,
            self,
            self.now(),
            tpl,
            props.chain((LVL_KEY, Level::Debug)),
        )
    }

    fn info<P: Props>(&self, tpl: Template, props: P) {
        base_emit(
            self,
            self,
            self,
            self.now(),
            tpl,
            props.chain((LVL_KEY, Level::Info)),
        )
    }

    fn warn<P: Props>(&self, tpl: Template, props: P) {
        base_emit(
            self,
            self,
            self,
            self.now(),
            tpl,
            props.chain((LVL_KEY, Level::Warn)),
        )
    }

    fn error<P: Props>(&self, tpl: Template, props: P) {
        base_emit(
            self,
            self,
            self,
            self.now(),
            tpl,
            props.chain((LVL_KEY, Level::Error)),
        )
    }

    fn debug_at<E: ToExtent, P: Props>(&self, extent: E, tpl: Template, props: P) {
        base_emit(
            self,
            self,
            self,
            extent,
            tpl,
            props.chain((LVL_KEY, Level::Debug)),
        )
    }

    fn info_at<E: ToExtent, P: Props>(&self, extent: E, tpl: Template, props: P) {
        base_emit(
            self,
            self,
            self,
            extent,
            tpl,
            props.chain((LVL_KEY, Level::Info)),
        )
    }

    fn warn_at<E: ToExtent, P: Props>(&self, extent: E, tpl: Template, props: P) {
        base_emit(
            self,
            self,
            self,
            extent,
            tpl,
            props.chain((LVL_KEY, Level::Warn)),
        )
    }

    fn error_at<E: ToExtent, P: Props>(&self, extent: E, tpl: Template, props: P) {
        base_emit(
            self,
            self,
            self,
            extent,
            tpl,
            props.chain((LVL_KEY, Level::Error)),
        )
    }

    fn push_ctxt<P: Props>(&self, props: P) -> Frame<&Self> {
        Frame::new(self, props)
    }

    fn current_ctxt(&self) -> Frame<&Self> {
        Frame::new(self, empty::Empty)
    }

    fn current_trace_id(&self) -> Option<TraceId> {
        let mut trace_id = None;

        self.with_current(|ctxt| {
            trace_id = ctxt.pull();
        });

        trace_id
    }

    fn current_span_id(&self) -> Option<SpanId> {
        let mut span_id = None;

        self.with_current(|ctxt| {
            span_id = ctxt.pull();
        });

        span_id
    }

    fn start_timer(&self) -> Timer<&Self> {
        Timer::start(self)
    }
}

impl<E: Emitter + Filter + Ctxt + Clock + Rng + ?Sized> Emit for E {}

#[doc(hidden)]
pub mod __private {
    pub use crate::macro_hooks::*;
    pub use core;
}
