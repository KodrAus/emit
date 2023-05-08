/*!
Emit structured events for programs and people.

`emit` is a front-end for capturing diagnostic data in programs and emitting them to
some outside observer. You can either configure `tracing` or your own function as the destination
for events.
*/

#![cfg_attr(not(feature = "std"), no_std)]

pub use emit_macros::*;
use time::Time;

mod macro_hooks;

mod adapt;
mod ambient;
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
    ambient::*,
    ctxt::{PropsCtxt, ScopeCtxt},
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
    with: impl PropsCtxt,
    lvl: Level,
    ts: Option<Timestamp>,
    tpl: Template,
    props: impl Props,
) {
    let ambient = ambient::get();

    with.chain(&ambient).with_props(|scope| {
        let ts = ts.or_else(|| ambient.timestamp());
        let evt = Event::new(lvl, ts, tpl, props.chain(scope));

        if when.chain(&ambient).matches_event(&evt) {
            to.chain(&ambient).emit_event(&evt);
        }
    })
}

#[cfg(feature = "std")]
pub fn ctxt() -> impl ScopeCtxt {
    ambient::get()
}

#[cfg(feature = "std")]
pub fn setup() -> Setup {
    Setup::default()
}

mod internal {
    pub struct Erased<T>(pub(crate) T);
}

#[doc(hidden)]
pub mod __private {
    pub use crate::macro_hooks::{__PrivateCaptureHook, __PrivateFmtHook};
    pub use core;
}
