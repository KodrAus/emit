#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

extern crate core;

pub mod ambient;
pub mod clock;
pub mod ctxt;
pub mod emitter;
pub mod empty;
pub mod event;
pub mod extent;
pub mod filter;
pub mod id;
pub mod key;
pub mod level;
pub mod props;
pub mod template;
pub mod timestamp;
pub mod value;
pub mod well_known;

mod internal {
    pub struct Erased<T>(pub(crate) T);
}
