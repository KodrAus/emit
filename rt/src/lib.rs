/*!
Implementation details for `emit!` macros.

This crate is not intended to be consumed directly.
*/

#![no_std]

#[cfg(any(feature = "std", test))]
#[macro_use]
#[allow(unused_imports)]
extern crate std;

#[cfg(not(any(feature = "std", test)))]
#[macro_use]
#[allow(unused_imports)]
extern crate core as std;

mod capture;
mod emit;
mod kvs;
mod record;
mod template;
mod value;

/**
This module is the entrypoint for the macros.

The code generated by the `ct` crate targets this module here in `rt`.
*/
#[doc(hidden)]
pub mod __private {
    pub use crate::{capture::__PrivateCapture, emit::*, kvs::*, record::*, template::*, value::*};
}
