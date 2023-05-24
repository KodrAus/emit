#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::future::Future;

pub use emit_macros::*;

mod macro_hooks;

mod ambient;
pub mod ctxt;
mod empty;
mod event;
pub mod filter;
pub mod id;
pub mod key;
mod level;
mod platform;
pub mod props;
pub mod target;
pub mod template;
pub mod time;
pub mod value;
pub mod well_known;

#[doc(inline)]
#[allow(unused_imports)]
pub use self::{
    ambient::*, event::*, id::Id, key::Key, level::*, props::Props, target::Target,
    template::Template, time::Timestamp, value::Value,
};

use self::{ctxt::Ctxt, filter::Filter, time::Clock};

pub fn emit(
    to: impl Target,
    when: impl Filter,
    with: impl Props,
    ts: Option<Timestamp>,
    id: Id,
    lvl: Level,
    tpl: Template,
    props: impl Props,
) {
    let ambient = ambient::get();

    ambient.with_current(|current_id, current_props| {
        let ts = ts.or_else(|| ambient.now());
        let id = id.or(current_id);
        let props = props.chain(with).chain(current_props);

        let evt = Event::new(ts, id, lvl, tpl, props);

        if when.matches_event(&evt) && ambient.matches_event(&evt) {
            to.emit_event(&evt);
            ambient.emit_event(&evt);
        }
    });
}

pub fn span<C: Ctxt>(ctxt: C, id: Id, props: impl Props) -> ctxt::Span<C> {
    let ambient = ambient::get();

    let id = id.or_gen(ctxt.current_id(), ambient);
    ctxt.span(id, props)
}

pub fn span_future<C: Ctxt, F: Future>(
    ctxt: C,
    id: Id,
    props: impl Props,
    future: F,
) -> ctxt::SpanFuture<C, F> {
    let ambient = ambient::get();

    let id = id.or_gen(ctxt.current_id(), ambient);

    ctxt.span_future(id, props, future)
}

#[cfg(feature = "std")]
pub fn target() -> impl Target {
    ambient::get()
}

#[cfg(feature = "std")]
pub fn filter() -> impl Filter {
    ambient::get()
}

#[cfg(feature = "std")]
pub fn ctxt() -> impl Ctxt {
    ambient::get()
}

#[cfg(feature = "std")]
pub fn clock() -> impl Clock {
    ambient::get()
}

#[cfg(feature = "std")]
pub fn gen_id() -> impl id::GenId {
    ambient::get()
}

#[cfg(feature = "std")]
pub fn setup() -> setup::Setup {
    setup::Setup::default()
}

mod internal {
    pub struct Erased<T>(pub(crate) T);
}

#[doc(hidden)]
pub mod __private {
    pub use crate::macro_hooks::*;
    pub use core;
}
