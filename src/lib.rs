/*!
Emit structured events for programs and people.

`emit` is a front-end for capturing diagnostic data in programs and emitting them to
some outside observer. You can either configure `tracing` or your own function as the destination
for events.
*/

#![cfg_attr(not(feature = "std"), no_std)]

pub use emit_macros::*;

mod macro_hooks;

mod adapt;
pub mod ctxt;
mod event;
pub mod filter;
mod key;
pub mod props;
pub mod target;
pub mod template;
pub mod time;
mod value;
pub mod well_known;

#[doc(inline)]
pub use self::{
    adapt::*, ctxt::Ctxt, event::*, filter::Filter, key::*, props::Props, target::Target,
    template::Template, time::Timestamp, value::*,
};

pub fn emit(
    to: impl Target,
    when: impl Filter,
    with: impl Ctxt,
    lvl: Level,
    ts: Option<Timestamp>,
    tpl: Template,
    props: impl Props,
) {
    let evt = Event::new(lvl, ts.or_else(now), tpl, props);

    with.chain(CTXT.get().by_ref()).with_props(|ctxt| {
        let evt = evt.by_ref().chain(ctxt);

        if when.chain(FILTER.get().by_ref()).matches_event(&evt) {
            to.chain(TARGET.get().by_ref()).emit_event(&evt);
        }
    })
}

mod internal {
    pub struct Erased<T>(pub(crate) T);
}

#[doc(hidden)]
pub mod __private {
    pub use crate::macro_hooks::{__PrivateCaptureHook, __PrivateFmtHook};
    pub use core;
}

#[cfg(feature = "std")]
use std::sync::OnceLock;

#[cfg(not(feature = "std"))]
struct StaticCell<T>(T);

#[cfg(not(feature = "std"))]
impl<T> StaticCell<T> {
    fn get(&self) -> &T {
        &self.0
    }
}

#[cfg(feature = "std")]
static TARGET: OnceLock<Box<dyn target::ErasedTarget + Send + Sync>> = OnceLock::new();

#[cfg(not(feature = "std"))]
static TARGET: StaticCell<target::Discard> = StaticCell(target::Discard);

#[cfg(feature = "std")]
pub fn to(target: impl Target + Send + Sync + 'static) {
    let _ = TARGET.set(Box::new(target));
}

#[cfg(feature = "std")]
static CTXT: OnceLock<Box<dyn ctxt::ErasedCtxt + Send + Sync>> = OnceLock::new();

#[cfg(not(feature = "std"))]
static CTXT: StaticCell<props::Empty> = StaticCell(props::Empty);

#[cfg(feature = "std")]
pub fn with(ctxt: impl Ctxt + Send + Sync + 'static) {
    let _ = CTXT.set(Box::new(ctxt));
}

#[cfg(feature = "std")]
static FILTER: OnceLock<Box<dyn filter::ErasedFilter + Send + Sync>> = OnceLock::new();

#[cfg(not(feature = "std"))]
static FILTER: StaticCell<filter::Always> = StaticCell(filter::Always);

#[cfg(feature = "std")]
pub fn when(filter: impl Filter + Send + Sync + 'static) {
    let _ = FILTER.set(Box::new(filter));
}

#[cfg(feature = "std")]
static TIME: OnceLock<Box<dyn time::Time + Send + Sync>> = OnceLock::new();

#[cfg(feature = "std")]
pub fn time(time: impl time::Time + Send + Sync + 'static) {
    let _ = TIME.set(Box::new(time));
}

fn now() -> Option<time::Timestamp> {
    #[cfg(feature = "std")]
    {
        Some(time::Timestamp::now())
    }
    #[cfg(not(feature = "std"))]
    {
        None
    }
}

/*
#[doc(inline)]
pub use emit_macros::*;

#[cfg(feature = "std")]
use crate::std::sync::OnceLock;

/**
A type that receives and emits event records.
*/
pub type Emitter = fn(&Event);

/**
The global implicit emitter.
*/
#[cfg(feature = "std")]
static EMITTER: OnceLock<Emitter> = OnceLock::new();

/**
Set the default target to emit to.
*/
#[cfg(feature = "std")]
pub fn to(emitter: Emitter) {
    drop(EMITTER.set(emitter));
}
*/
