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

#[cfg(feature = "std")]
mod setup;

pub fn emit(to: impl Target, when: impl Filter, lvl: Level, tpl: Template, props: impl Props) {
    let ambient = emit_core::ambient::get();

    ambient.with_current(|id, current_props| {
        let ts = ambient.now();
        let props = props.chain(current_props);

        let evt = Event::new(ts, id, lvl, tpl, props);

        if when.matches_event(&evt) && ambient.matches_event(&evt) {
            to.emit_event(&evt);
            ambient.emit_event(&evt);
        }
    });
}

pub fn span(
    id: Id,
    tpl: Template,
    props: impl Props,
) -> ctxt::Span<impl Ctxt + Send + Sync + 'static, impl Clock + Send + Sync + 'static> {
    let ambient = emit_core::ambient::get();

    let id = id.or_gen(ambient.current_id(), ambient);
    ctxt::Span::new(ambient, ambient, id, tpl, props)
}

pub fn span_future<F: Future>(
    id: Id,
    tpl: Template,
    props: impl Props,
    future: F,
) -> ctxt::SpanFuture<impl Ctxt + Send + Sync + 'static, F, impl Clock + Send + Sync + 'static> {
    let ambient = emit_core::ambient::get();

    let id = id.or_gen(ambient.current_id(), ambient);
    ctxt::SpanFuture::new(ambient, ambient, id, tpl, props, future)
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
