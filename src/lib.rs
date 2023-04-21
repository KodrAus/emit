/*!
Emit structured events for programs and people.

`emit` is a front-end for capturing diagnostic data in programs and emitting them to
some outside observer. You can either configure `tracing` or your own function as the destination
for events.
*/

/*
#![no_std]

#[cfg(any(feature = "std", test))]
#[macro_use]
#[allow(unused_imports)]
extern crate std;

#[cfg(not(any(feature = "std", test)))]
#[macro_use]
#[allow(unused_imports)]
extern crate core as std;
*/

mod capture;

pub mod ctxt;
mod event;
pub mod props;
mod template;
pub mod to;
mod value;
pub mod well_known;
pub mod when;

use std::cell::RefCell;

#[doc(inline)]
pub use self::{
    ctxt::Ctxt,
    event::*,
    props::{Prop, Props},
    template::*,
    to::To,
    value::*,
    when::When,
};

pub fn emit(emitter: impl To, filter: impl When, ctxt: impl Ctxt, evt: Event<impl Props>) {
    ctxt.with_ctxt(|ctxt| {
        let evt = evt.chain(ctxt);

        if filter.emit_when(&evt) {
            let _ = emitter.emit_to(evt);
        }
    })
}

fn check(evt: Event<impl Props>) {
    emit(
        to::default(),
        when::default(),
        ctxt::default().by_ref().chain(ctxt::default()),
        evt,
    );
}

struct ThreadLocalCtxt;

struct ThreadLocalProps;

impl Props for ThreadLocalProps {
    fn visit<'a, V: props::Visit<'a>>(&'a self, visitor: V) {}
}

impl Ctxt for ThreadLocalCtxt {
    type Props = ThreadLocalProps;

    fn with_ctxt<F: FnOnce(&Self::Props)>(&self, with: F) {
        thread_local! {
            static CTXT: RefCell<ThreadLocalProps> = RefCell::new(ThreadLocalProps);
        }

        CTXT.with(|ctxt| with(&*ctxt.borrow()))
    }
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
