#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::future::Future;

use emit_core::{ctxt::Ctxt, filter::Filter, target::Target, time::Clock};

#[doc(inline)]
pub use emit_macros::*;

#[doc(inline)]
pub use emit_core::{
    ctxt, empty, event, filter, id, key, level, props, target, template, time, value, well_known,
};

pub use self::{
    event::Event, id::Id, key::Key, level::Level, props::Props, template::Template,
    time::Timestamp, value::Value,
};

mod macro_hooks;
mod platform;
mod setup;

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
    let ambient = emit_core::ambient::get();

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
    let ambient = emit_core::ambient::get();

    let id = id.or_gen(ctxt.current_id(), ambient);
    ctxt.span(id, props)
}

pub fn span_future<C: Ctxt, F: Future>(
    ctxt: C,
    id: Id,
    props: impl Props,
    future: F,
) -> ctxt::SpanFuture<C, F> {
    let ambient = emit_core::ambient::get();

    let id = id.or_gen(ctxt.current_id(), ambient);

    ctxt.span_future(id, props, future)
}

#[cfg(feature = "std")]
pub fn setup() -> setup::Setup {
    setup::Setup::default()
}

#[cfg(feature = "std")]
pub fn target() -> impl Target {
    emit_core::ambient::get()
}

#[cfg(feature = "std")]
pub fn filter() -> impl Filter {
    emit_core::ambient::get()
}

#[cfg(feature = "std")]
pub fn ctxt() -> impl Ctxt {
    emit_core::ambient::get()
}

#[cfg(feature = "std")]
pub fn clock() -> impl Clock {
    emit_core::ambient::get()
}

#[cfg(feature = "std")]
pub fn gen_id() -> impl id::GenId {
    emit_core::ambient::get()
}

#[doc(hidden)]
pub mod __private {
    pub use crate::macro_hooks::*;
    pub use core;
}
