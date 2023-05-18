#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub use emit_macros::*;

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
    ambient::*, event::*, key::*, props::Props, target::Target, template::Template,
    time::Timestamp, value::*,
};

use self::{ctxt::Ctxt, filter::Filter, time::Time};

pub fn emit(
    to: impl Target,
    when: impl Filter,
    with: impl Props,
    ts: impl Time,
    lvl: Level,
    tpl: Template,
    props: impl Props,
) {
    let ambient = ambient::get();

    ambient.with_props(|ctxt| {
        let props = props.chain(with).chain(ctxt);

        let ts = ts.timestamp().or_else(|| ambient.timestamp());

        let evt = Event::new(ts, lvl, tpl, props);

        if when.matches_event(&evt) && ambient.matches_event(&evt) {
            to.emit_event(&evt);
            ambient.emit_event(&evt);
        }
    });
}

#[cfg(feature = "std")]
pub fn ctxt() -> impl Ctxt {
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
    pub use crate::macro_hooks::*;
    pub use core;
}
