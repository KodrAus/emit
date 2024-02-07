#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use emit_core::{extent::ToExtent, well_known::LVL_KEY};

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
    frame::FrameCtxt,
    id::{IdCtxt, IdRng, SpanId, TraceId},
    level::Level,
    props::Props,
    rng::Rng,
    str::Str,
    template::Template,
    timer::{StartTimer, Timer},
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
}

impl<E: Emitter + Filter + Ctxt + Clock + Rng + ?Sized> Emit for E {}

#[doc(hidden)]
pub mod __private {
    pub use crate::macro_hooks::*;
    pub use core;
}
