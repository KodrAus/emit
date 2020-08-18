/*!
Implementation details for log macro expansion.

This crate is not intended to be consumed directly.
*/

#![feature(min_const_generics)] // could be replaced by a concrete `32`
#![feature(min_specialization)] // required to accept `T: Sized + 'static || dyn Trait || str`
#![feature(extern_types)] // could be replaced by empty enums

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
mod log;
mod source;
mod template;
mod value;

#[doc(hidden)]
pub mod __private {
    pub use crate::{capture::__PrivateCapture, log::*, source::*, template::*, value::*};
}
