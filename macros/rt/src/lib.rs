/*!
Implementation details for log macro expansion.

This crate is not intended to be consumed directly.
*/

#![feature(min_specialization)] // required to accept `T: Sized + 'static || dyn Trait || str`
#![feature(extern_types)] // could be replaced by empty enums

mod capture;
mod source;

#[doc(hidden)]
pub mod __private {
    pub use crate::{capture::__PrivateLogCapture, source::Captured};

    pub use log::kv::{Key, Value};
}
