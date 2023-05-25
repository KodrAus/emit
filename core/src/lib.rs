#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod ambient;
pub mod ctxt;
pub mod empty;
pub mod event;
pub mod filter;
pub mod id;
pub mod key;
pub mod level;
pub mod props;
pub mod target;
pub mod template;
pub mod time;
pub mod value;
pub mod well_known;

mod internal {
    pub struct Erased<T>(pub(crate) T);
}
