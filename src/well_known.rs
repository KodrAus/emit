use std::fmt;

use crate::value::Val;

/**
The semantic `err` property value.
*/
#[repr(transparent)]
pub struct Err<'a>(Val<'a>);

impl<'a> Err<'a> {
    pub const KEY: &'static str = "err";

    pub fn new(value: Val<'a>) -> Self {
        Err(value)
    }

    pub fn new_ref<'b>(value: &'b Val<'a>) -> &'b Self {
        unsafe { &*(value as *const Val<'a> as *const Err<'a>) }
    }
}
