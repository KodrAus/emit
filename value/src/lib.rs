//! Structured values.

// TODO: This will be necessary after https://github.com/rust-lang/rust/pull/77083 hits nightly
// #![cfg_attr(value_bag_const_type_id, feature(const_type_id))]

#![no_std]

#[cfg(any(feature = "std", test))]
#[macro_use]
#[allow(unused_imports)]
extern crate std;

#[cfg(not(any(feature = "std", test)))]
#[macro_use]
#[allow(unused_imports)]
extern crate core as std;

mod error;
mod fill;
mod impls;
mod internal;

#[cfg(test)]
mod test;

pub use self::{
    error::Error,
    fill::{Fill, Slot},
};

use self::internal::{Inner, Primitive, Visitor};

/// A dynamic structured value.
///
/// # Capturing values
///
/// There are a few ways to capture a value:
///
/// - Using the `ValueBag::capture_*` methods.
/// - Using the standard `From` trait.
/// - Using the `Fill` API.
///
/// ## Using the `ValueBag::capture_*` methods
///
/// `ValueBag` offers a few constructor methods that capture values of different kinds.
/// These methods require a `T: 'static` to support downcasting.
///
/// ```
/// use value_bag::ValueBag;
///
/// let value = ValueBag::capture_debug(&42i32);
///
/// assert_eq!(Some(42), value.to_i32());
/// ```
///
/// ## Using the standard `From` trait
///
/// Standard types that implement `ToValue` also implement `From`.
///
/// ```
/// use value_bag::ValueBag;
///
/// let value = ValueBag::from(42i32);
///
/// assert_eq!(Some(42), value.to_i32());
/// ```
///
/// ```
/// # use std::fmt::Debug;
/// use value_bag::ValueBag;
///
/// let value = ValueBag::from(&42i32 as &dyn Debug);
///
/// assert_eq!(None, value.to_i32());
/// ```
///
/// ## Using the `Fill` API
///
/// The `Fill` trait is a way to bridge APIs that may not be directly
/// compatible with other constructor methods.
///
/// ```
/// use value_bag::{ValueBag, Slot, Fill, Error};
///
/// struct FillSigned;
///
/// impl Fill for FillSigned {
///     fn fill(&self, slot: &mut Slot) -> Result<(), Error> {
///         slot.fill_any(42i32)
///     }
/// }
///
/// let value = ValueBag::from_fill(&FillSigned);
///
/// assert_eq!(Some(42), value.to_i32());
/// ```
///
/// ```
/// # use std::fmt::Debug;
/// use value_bag::{ValueBag, Slot, Fill, Error};
///
/// struct FillDebug;
///
/// impl Fill for FillDebug {
///     fn fill(&self, slot: &mut Slot) -> Result<(), Error> {
///         slot.fill_debug(&42i32 as &dyn Debug)
///     }
/// }
///
/// let value = ValueBag::from_fill(&FillDebug);
///
/// assert_eq!(None, value.to_i32());
/// ```
///
/// # Inspecting values
///
/// Once you have a `ValueBag` there are also a few ways to inspect it:
///
/// - Using the `Debug`, `Display`, `Serialize`, and `Stream` trait implementations.
/// - Using the `ValueBag::to_*` methods.
/// - Using the `ValueBag::downcast_ref` method.
#[derive(Clone)]
pub struct ValueBag<'v> {
    inner: Inner<'v>,
}

impl<'v> ValueBag<'v> {
    /// Get a value from an internal primitive.
    fn from_primitive<T>(value: T) -> Self
    where
        T: Into<Primitive<'v>>,
    {
        ValueBag {
            inner: Inner::Primitive {
                value: value.into(),
            },
        }
    }

    /// Visit the value using an internal visitor.
    fn visit<'a>(&'a self, visitor: &mut dyn Visitor<'a>) -> Result<(), Error> {
        self.inner.visit(visitor)
    }
}
