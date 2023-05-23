#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::future::Future;

pub use emit_macros::*;
use id::IdGenerator;

mod macro_hooks;

mod ambient;
pub mod ctxt;
mod empty;
mod event;
pub mod filter;
pub mod id;
mod key;
mod platform;
pub mod props;
pub mod target;
pub mod template;
pub mod time;
mod value;
pub mod well_known;

#[doc(inline)]
#[allow(unused_imports)]
pub use self::{
    ambient::*, event::*, id::Id, key::*, props::Props, target::Target, template::Template,
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

    ambient.with_current(|id, ctxt| {
        let props = props.chain(with).chain(ctxt);

        let ts = ts.timestamp().or_else(|| ambient.timestamp());

        let evt = Event::new(ts, id, lvl, tpl, props);

        if when.matches_event(&evt) && ambient.matches_event(&evt) {
            to.emit_event(&evt);
            ambient.emit_event(&evt);
        }
    });
}

pub fn span<C: Ctxt>(ctxt: C, id: impl IdGenerator, props: impl Props) -> ctxt::Span<C> {
    let id = ctxt.current_id().merge(Id::new(id.trace(), id.span()));
    ctxt.span(id, props)
}

pub fn span_future<C: Ctxt, F: Future>(
    ctxt: C,
    id: impl IdGenerator,
    props: impl Props,
    future: F,
) -> ctxt::SpanFuture<C, F> {
    let id = ctxt.current_id().merge(Id::new(id.trace(), id.span()));

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
pub fn time() -> impl Time {
    ambient::get()
}

#[cfg(feature = "std")]
pub fn id_generator() -> impl IdGenerator {
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
