#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

extern crate core;

pub mod clock;
pub mod ctxt;
pub mod emitter;
pub mod empty;
pub mod event;
pub mod extent;
pub mod filter;
pub mod props;
pub mod rng;
pub mod runtime;
pub mod str;
pub mod template;
pub mod timestamp;
pub mod value;
pub mod well_known;

mod internal {
    pub struct Erased<T>(pub(crate) T);
}
