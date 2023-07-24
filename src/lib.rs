#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::future::Future;

use emit_core::{ctxt::Ctxt, filter::Filter, target::Target, time::Clock};

#[doc(inline)]
pub use emit_macros::*;

#[doc(inline)]
pub use emit_core::{
    empty, event, filter, id, key, level, props, target, template, time, value, well_known,
};

pub mod ctxt {
    pub use crate::local_frame::*;
    #[doc(inline)]
    pub use emit_core::ctxt::*;
}

use emit_core::{value::ToValue, well_known::WellKnown};

pub use self::{
    event::Event, key::Key, level::Level, props::Props, template::Template, time::Timestamp,
    value::Value,
};

use crate::ctxt::{LocalFrame, LocalFrameFuture};

mod macro_hooks;
mod platform;

pub mod local_frame;

#[cfg(feature = "std")]
mod setup;

#[track_caller]
pub fn emit(evt: &Event<impl Props>) {
    let ambient = emit_core::ambient::get();

    base_emit(
        ambient,
        ambient,
        ambient,
        ambient,
        evt.template(),
        evt.props(),
    );
}

#[track_caller]
fn base_emit(
    to: impl Target,
    when: impl Filter,
    ctxt: impl Ctxt,
    clock: impl Clock,
    tpl: Template,
    props: impl Props,
) {
    ctxt.with_current(|ctxt| {
        let ts = clock.now();

        let evt = Event::point(ts, tpl, props.chain(ctxt));

        if when.matches(&evt) {
            to.event(&evt);
        }
    });
}

#[track_caller]
fn base_with(ctxt: impl Ctxt, props: impl Props) -> LocalFrame<impl Ctxt> {
    LocalFrame::new(ctxt, props)
}

#[track_caller]
fn base_with_future<F: Future>(
    ctxt: impl Ctxt + Send + Sync + 'static,
    props: impl Props,
    future: F,
) -> LocalFrameFuture<impl Ctxt + Send + Sync + 'static, F> {
    LocalFrameFuture::new(ctxt, props, future)
}

#[cfg(feature = "std")]
pub fn setup() -> setup::Setup {
    setup::Setup::default()
}

#[doc(hidden)]
pub mod __private {
    pub use crate::macro_hooks::*;
    pub use core;
}
