/*!
Emit structured events for programs and people.

`emit` is a front-end for capturing diagnostic data in programs and emitting them to
some outside observer. You can either configure `tracing` or your own function as the destination
for events.
*/

#![cfg_attr(not(feature = "std"), no_std)]

mod capture;

pub mod ctxt;
mod event;
pub mod key;
pub mod props;
mod template;
pub mod to;
pub mod value;
pub mod well_known;
pub mod when;

#[doc(inline)]
pub use self::{
    ctxt::Ctxt, event::*, key::*, props::Props, template::*, to::To, value::*, when::When,
};

pub fn emit(emitter: impl To, filter: impl When, ctxt: impl Ctxt, evt: &Event<impl Props>) {
    ctxt.with_ctxt(|ctxt| {
        let evt = evt.by_ref().chain(ctxt);

        if filter.emit_when(&evt) {
            let _ = emitter.emit_to(&evt);
        }
    })
}

mod internal {
    pub struct Erased<T>(pub(crate) T);
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
