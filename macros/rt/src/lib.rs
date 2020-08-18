/*!
Implementation details for log macro expansion.

This crate is not intended to be consumed directly.
*/

#![feature(
    maybe_uninit_uninit_array,
    maybe_uninit_slice,
    maybe_uninit_slice_assume_init
)] // could be replaced by some manual code
#![feature(min_const_generics)] // could be replaced by a concrete `32`
#![feature(min_specialization)] // required to accept `T: Sized + 'static || dyn Trait || str`
#![feature(extern_types)] // could be replaced by empty enums

mod capture;
mod log;
mod source;
mod template;
mod value;

#[doc(hidden)]
pub mod __private {
    pub use crate::{capture::__PrivateCapture, log::*, source::*, template::*, value::*};
}
