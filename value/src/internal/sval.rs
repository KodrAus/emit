//! Integration between `Value` and `sval`.
//!
//! This module allows any `Value` to implement the `Value` trait,
//! and for any `Value` to be captured as a `Value`.

use crate::{std::fmt, Error, Slot, ValueBag};

use super::{
    cast::{self, Cast},
    Inner, Primitive, Visitor,
};

impl<'v> ValueBag<'v> {
    /// Get a value from a structured type.
    ///
    /// This method will attempt to capture the given value as a well-known primitive
    /// before resorting to using its `Value` implementation.
    pub fn capture_sval<T>(value: &'v T) -> Self
    where
        T: Value + 'static,
    {
        cast::try_from_primitive(value).unwrap_or(ValueBag {
            inner: Inner::Sval {
                value,
                type_id: Some(cast::type_id::<T>()),
            },
        })
    }
}

impl<'s, 'f> Slot<'s, 'f> {
    /// Fill the slot with a structured value.
    ///
    /// The given value doesn't need to satisfy any particular lifetime constraints.
    ///
    /// # Panics
    ///
    /// Calling more than a single `fill` method on this slot will panic.
    pub fn fill_sval<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: Value,
    {
        self.fill(|visitor| visitor.sval(&value))
    }
}

impl<'v> Value for ValueBag<'v> {
    fn stream(&self, s: &mut sval::value::Stream) -> sval::value::Result {
        struct SvalVisitor<'a, 'b: 'a>(&'a mut sval::value::Stream<'b>);

        impl<'a, 'b: 'a, 'v> Visitor<'v> for SvalVisitor<'a, 'b> {
            fn debug(&mut self, v: &dyn fmt::Debug) -> Result<(), Error> {
                self.0.debug(v).map_err(Error::from_sval)
            }

            fn u64(&mut self, v: u64) -> Result<(), Error> {
                self.0.u64(v).map_err(Error::from_sval)
            }

            fn i64(&mut self, v: i64) -> Result<(), Error> {
                self.0.i64(v).map_err(Error::from_sval)
            }

            fn f64(&mut self, v: f64) -> Result<(), Error> {
                self.0.f64(v).map_err(Error::from_sval)
            }

            fn bool(&mut self, v: bool) -> Result<(), Error> {
                self.0.bool(v).map_err(Error::from_sval)
            }

            fn char(&mut self, v: char) -> Result<(), Error> {
                self.0.char(v).map_err(Error::from_sval)
            }

            fn str(&mut self, v: &str) -> Result<(), Error> {
                self.0.str(v).map_err(Error::from_sval)
            }

            fn none(&mut self) -> Result<(), Error> {
                self.0.none().map_err(Error::from_sval)
            }

            #[cfg(feature = "std")]
            fn error(&mut self, v: &dyn std::error::Error) -> Result<(), Error> {
                Value::stream(&sval::stream::Source::from(v), self.0).map_err(Error::from_sval)
            }

            fn sval(&mut self, v: &dyn Value) -> Result<(), Error> {
                self.0.any(v).map_err(Error::from_sval)
            }

            #[cfg(feature = "serde")]
            fn serde(&mut self, v: &dyn super::serde::Serialize) -> Result<(), Error> {
                super::serde::sval(self.0, v)
            }
        }

        self.visit(&mut SvalVisitor(s)).map_err(Error::into_sval)?;

        Ok(())
    }
}

impl<'v> From<&'v (dyn Value)> for ValueBag<'v> {
    fn from(value: &'v (dyn Value)) -> ValueBag<'v> {
        ValueBag {
            inner: Inner::Sval {
                value,
                type_id: None,
            },
        }
    }
}

pub use sval::value::Value;

pub(super) fn fmt(f: &mut fmt::Formatter, v: &dyn Value) -> Result<(), Error> {
    sval::fmt::debug(f, v)?;
    Ok(())
}

#[cfg(feature = "serde")]
pub(super) fn serde<S>(s: S, v: &dyn Value) -> Result<S::Ok, S::Error>
where
    S: serde_lib::Serializer,
{
    sval::serde::v1::serialize(s, v)
}

pub(super) fn cast<'v>(v: &dyn Value) -> Cast<'v> {
    struct CastStream<'v>(Cast<'v>);

    impl<'v> sval::stream::Stream for CastStream<'v> {
        fn u64(&mut self, v: u64) -> sval::stream::Result {
            self.0 = Cast::Primitive(Primitive::from(v));
            Ok(())
        }

        fn i64(&mut self, v: i64) -> sval::stream::Result {
            self.0 = Cast::Primitive(Primitive::from(v));
            Ok(())
        }

        fn f64(&mut self, v: f64) -> sval::stream::Result {
            self.0 = Cast::Primitive(Primitive::from(v));
            Ok(())
        }

        fn char(&mut self, v: char) -> sval::stream::Result {
            self.0 = Cast::Primitive(Primitive::from(v));
            Ok(())
        }

        fn bool(&mut self, v: bool) -> sval::stream::Result {
            self.0 = Cast::Primitive(Primitive::from(v));
            Ok(())
        }

        #[cfg(feature = "std")]
        fn str(&mut self, s: &str) -> sval::stream::Result {
            self.0 = Cast::String(s.into());
            Ok(())
        }
    }

    let mut cast = CastStream(Cast::Primitive(Primitive::None));
    let _ = sval::stream(&mut cast, v);

    cast.0
}

impl Error {
    pub(crate) fn from_sval(_: sval::Error) -> Self {
        Error::msg("`sval` serialization failed")
    }

    pub(crate) fn into_sval(self) -> sval::Error {
        sval::Error::msg("`sval` serialization failed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::Token;

    #[test]
    fn sval_capture() {
        assert_eq!(ValueBag::capture_sval(&42u64).to_token(), Token::U64(42));
    }

    #[test]
    fn sval_cast() {
        assert_eq!(
            42u32,
            ValueBag::capture_sval(&42u64)
                .to_u32()
                .expect("invalid value")
        );

        assert_eq!(
            "a string",
            ValueBag::capture_sval(&"a string")
                .to_borrowed_str()
                .expect("invalid value")
        );

        #[cfg(feature = "std")]
        assert_eq!(
            "a string",
            ValueBag::capture_sval(&"a string")
                .to_str()
                .expect("invalid value")
        );
    }

    #[test]
    fn sval_downcast() {
        #[derive(Debug, PartialEq, Eq)]
        struct Timestamp(usize);

        impl Value for Timestamp {
            fn stream(&self, stream: &mut sval::value::Stream) -> sval::value::Result {
                stream.u64(self.0 as u64)
            }
        }

        let ts = Timestamp(42);

        assert_eq!(
            &ts,
            ValueBag::capture_sval(&ts)
                .downcast_ref::<Timestamp>()
                .expect("invalid value")
        );
    }

    #[test]
    fn sval_structured() {
        let value = ValueBag::from(42u64);
        let expected = vec![sval::test::Token::Unsigned(42)];

        assert_eq!(sval::test::tokens(value), expected);
    }

    #[test]
    fn sval_debug() {
        struct TestSval;

        impl Value for TestSval {
            fn stream(&self, stream: &mut sval::value::Stream) -> sval::value::Result {
                stream.u64(42)
            }
        }

        assert_eq!(
            format!("{:04?}", 42u64),
            format!("{:04?}", ValueBag::capture_sval(&TestSval)),
        );
    }

    #[test]
    #[cfg(feature = "serde")]
    fn sval_serde() {
        use serde_test::{assert_ser_tokens, Token};

        struct TestSval;

        impl Value for TestSval {
            fn stream(&self, stream: &mut sval::value::Stream) -> sval::value::Result {
                stream.u64(42)
            }
        }

        assert_ser_tokens(&ValueBag::capture_sval(&TestSval), &[Token::U64(42)]);
    }

    #[cfg(feature = "std")]
    mod std_support {
        use super::*;

        use crate::std::borrow::ToOwned;

        #[test]
        fn sval_cast() {
            assert_eq!(
                "a string",
                ValueBag::capture_sval(&"a string".to_owned())
                    .to_str()
                    .expect("invalid value")
            );
        }
    }
}
