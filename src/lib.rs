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
    adapt::*,
    ctxt::{GetCtxt, SetCtxt},
    event::*,
    filter::Filter,
    key::*,
    props::Props,
    target::Target,
    template::Template,
    time::Timestamp,
    value::*,
};

pub fn emit(
    to: impl Target,
    when: impl Filter,
    with: impl GetCtxt,
    lvl: Level,
    ts: Option<Timestamp>,
    tpl: Template,
    props: impl Props,
) {
    let evt = Event::new(lvl, ts.or_else(now), tpl, props);

    with.chain(GET_CTXT.get().by_ref()).with_props(|ctxt| {
        let evt = evt.chain(ctxt);

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
use std::sync::{Arc, OnceLock};

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
static GET_CTXT: OnceLock<Arc<dyn ctxt::ErasedGetCtxt + Send + Sync>> = OnceLock::new();

#[cfg(not(feature = "std"))]
static GET_CTXT: StaticCell<props::Empty> = StaticCell(props::Empty);

#[cfg(feature = "std")]
pub fn with(ctxt: impl GetCtxt + Send + Sync + 'static) {
    let _ = GET_CTXT.set(Arc::new(ctxt));
}

#[cfg(feature = "std")]
static SET_CTXT: OnceLock<Arc<dyn ctxt::ErasedSetCtxt + Send + Sync>> = OnceLock::new();

#[cfg(feature = "std")]
pub fn with_dyanmic(ctxt: impl GetCtxt + SetCtxt + Send + Sync + 'static) {
    let ctxt = Arc::new(ctxt);
    let _ = GET_CTXT.set(ctxt.clone());
    let _ = SET_CTXT.set(ctxt.clone());
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
