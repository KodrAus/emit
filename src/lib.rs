/*!
Emit structured events for programs and people.

`emit` is a front-end for capturing diagnostic data in programs and emitting them to
some outside observer. You can either configure `tracing` or your own function as the destination
for events.
*/

#![cfg_attr(feature = "std", feature(once_cell))]
#![no_std]

#[cfg(any(feature = "std", test))]
#[macro_use]
#[allow(unused_imports)]
extern crate std;

#[cfg(not(any(feature = "std", test)))]
#[macro_use]
#[allow(unused_imports)]
extern crate core as std;

/**
Macros for emitting log events.
*/
pub use emit_ct::{
    as_debug, as_display, as_serde, as_sval, debug, emit, error, format, info, source, trace, warn,
};

/**
Private entrypoint for the `ct` crate.

Code generation expects to find items at `emit::ct::__private`, not `emit_ct::__private`.
*/
#[doc(hidden)]
pub use emit_ct as ct;

/**
Private entrypoint for the `rt` crate.

Code generation expects to find items at `emit::rt::__private`, not `emit_rt::__private`.
*/
#[doc(hidden)]
pub use emit_rt as rt;

mod emit;
mod record;

#[cfg(feature = "std")]
use crate::std::sync::OnceLock;

/**
A type that receives and emits event records.
*/
pub type Emitter = fn(&Record);

/**
The global implicit emitter.
*/
#[cfg(feature = "std")]
static EMITTER: OnceLock<Emitter> = OnceLock::new();

/**
Set the default target to emit to.
*/
#[cfg(feature = "std")]
pub fn target(emitter: Emitter) {
    drop(EMITTER.set(emitter));
}

#[doc(inline)]
pub use record::*;

/**
Private entrypoint for the `emit` crate.

Code generation expects to find items at `emit::__private`.
*/
#[doc(hidden)]
pub mod __private {
    pub use crate::emit::*;
}
