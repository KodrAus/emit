use crate::{
    std::{fmt, marker::PhantomData},
    Error, Slot, ValueBag,
};

use super::{
    cast::{self, Cast},
    Inner, Primitive, Visitor,
};

use serde_lib::ser::{Error as SerdeError, Impossible};

impl<'v> ValueBag<'v> {
    /// Get a value from a structured type.
    ///
    /// This method will attempt to capture the given value as a well-known primitive
    /// before resorting to using its `Value` implementation.
    pub fn capture_serde<T>(value: &'v T) -> Self
    where
        T: Serialize + 'static,
    {
        cast::try_from_primitive(value).unwrap_or(ValueBag {
            inner: Inner::Serde {
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
    pub fn fill_serde<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: Serialize,
    {
        self.fill(|visitor| visitor.serde(&value))
    }
}

impl<'v> serde_lib::Serialize for ValueBag<'v> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde_lib::Serializer,
    {
        struct SerdeVisitor<S>
        where
            S: serde_lib::Serializer,
        {
            inner: Option<S>,
            result: Option<Result<S::Ok, S::Error>>,
        }

        impl<S> SerdeVisitor<S>
        where
            S: serde_lib::Serializer,
        {
            fn result(&self) -> Result<(), Error> {
                match self.result {
                    Some(Ok(_)) => Ok(()),
                    Some(Err(_)) | None => Err(Error::serde()),
                }
            }

            fn serializer(&mut self) -> Result<S, Error> {
                self.inner.take().ok_or_else(|| Error::serde())
            }

            fn into_result(self) -> Result<S::Ok, S::Error> {
                self.result
                    .unwrap_or_else(|| Err(S::Error::custom("`serde` serialization failed")))
            }
        }

        impl<'v, S> Visitor<'v> for SerdeVisitor<S>
        where
            S: serde_lib::Serializer,
        {
            fn debug(&mut self, v: &dyn fmt::Debug) -> Result<(), Error> {
                struct DebugToDisplay<T>(T);

                impl<T> fmt::Display for DebugToDisplay<T>
                where
                    T: fmt::Debug,
                {
                    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        fmt::Debug::fmt(&self.0, f)
                    }
                }

                self.result = Some(self.serializer()?.collect_str(&DebugToDisplay(v)));
                self.result()
            }

            fn u64(&mut self, v: u64) -> Result<(), Error> {
                self.result = Some(self.serializer()?.serialize_u64(v));
                self.result()
            }

            fn i64(&mut self, v: i64) -> Result<(), Error> {
                self.result = Some(self.serializer()?.serialize_i64(v));
                self.result()
            }

            fn f64(&mut self, v: f64) -> Result<(), Error> {
                self.result = Some(self.serializer()?.serialize_f64(v));
                self.result()
            }

            fn bool(&mut self, v: bool) -> Result<(), Error> {
                self.result = Some(self.serializer()?.serialize_bool(v));
                self.result()
            }

            fn char(&mut self, v: char) -> Result<(), Error> {
                self.result = Some(self.serializer()?.serialize_char(v));
                self.result()
            }

            fn str(&mut self, v: &str) -> Result<(), Error> {
                self.result = Some(self.serializer()?.serialize_str(v));
                self.result()
            }

            fn none(&mut self) -> Result<(), Error> {
                self.result = Some(self.serializer()?.serialize_none());
                self.result()
            }

            #[cfg(feature = "std")]
            fn error(&mut self, v: &dyn std::error::Error) -> Result<(), Error> {
                self.result = Some(self.serializer()?.collect_str(v));
                self.result()
            }

            #[cfg(feature = "sval")]
            fn sval(&mut self, v: &dyn super::sval::Value) -> Result<(), Error> {
                self.result = Some(super::sval::serde(self.serializer()?, v));
                self.result()
            }

            fn serde(&mut self, v: &dyn Serialize) -> Result<(), Error> {
                self.result = Some(erased_serde::serialize(v, self.serializer()?));
                self.result()
            }
        }

        let mut visitor = SerdeVisitor {
            inner: Some(s),
            result: None,
        };

        self.visit(&mut visitor).map_err(|e| S::Error::custom(e))?;

        visitor.into_result()
    }
}

impl<'v> From<&'v (dyn Serialize)> for ValueBag<'v> {
    fn from(value: &'v (dyn Serialize)) -> ValueBag<'v> {
        ValueBag {
            inner: Inner::Serde {
                value,
                type_id: None,
            },
        }
    }
}

pub use erased_serde::Serialize;

pub(super) fn fmt(f: &mut fmt::Formatter, v: &dyn Serialize) -> Result<(), Error> {
    fmt::Debug::fmt(&serde_fmt::to_debug(v), f)?;
    Ok(())
}

#[cfg(feature = "sval")]
pub(super) fn sval(s: &mut sval::value::Stream, v: &dyn Serialize) -> Result<(), Error> {
    sval::serde::v1::stream(s, v).map_err(Error::from_sval)?;
    Ok(())
}

pub(super) fn cast<'v>(v: &dyn Serialize) -> Cast<'v> {
    struct CastSerializer<'v>(PhantomData<Cast<'v>>);

    impl<'v> serde_lib::Serializer for CastSerializer<'v> {
        type Ok = Cast<'v>;
        type Error = InvalidCast;

        type SerializeSeq = Impossible<Self::Ok, Self::Error>;
        type SerializeTuple = Impossible<Self::Ok, Self::Error>;
        type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
        type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
        type SerializeMap = Impossible<Self::Ok, Self::Error>;
        type SerializeStruct = Impossible<Self::Ok, Self::Error>;
        type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

        fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::from(v)))
        }

        fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::from(v)))
        }

        fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::from(v)))
        }

        fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::from(v)))
        }

        fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::from(v)))
        }

        fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::from(v)))
        }

        fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::from(v)))
        }

        fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::from(v)))
        }

        fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::from(v)))
        }

        fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::from(v)))
        }

        fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::from(v)))
        }

        fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::from(v)))
        }

        fn serialize_some<T>(self, v: &T) -> Result<Self::Ok, Self::Error>
        where
            T: serde_lib::Serialize + ?Sized,
        {
            v.serialize(self)
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::None))
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::Primitive(Primitive::None))
        }

        fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
            Err(InvalidCast)
        }

        #[cfg(not(feature = "std"))]
        fn serialize_str(self, _: &str) -> Result<Self::Ok, Self::Error> {
            Err(InvalidCast)
        }

        #[cfg(feature = "std")]
        fn serialize_str(self, s: &str) -> Result<Self::Ok, Self::Error> {
            Ok(Cast::String(s.into()))
        }

        fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
            Err(InvalidCast)
        }

        fn serialize_unit_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            Err(InvalidCast)
        }

        fn serialize_newtype_struct<T>(
            self,
            _: &'static str,
            _: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: serde_lib::Serialize + ?Sized,
        {
            Err(InvalidCast)
        }

        fn serialize_newtype_variant<T>(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: serde_lib::Serialize + ?Sized,
        {
            Err(InvalidCast)
        }

        fn serialize_seq(
            self,
            _: core::option::Option<usize>,
        ) -> Result<Self::SerializeSeq, Self::Error> {
            Err(InvalidCast)
        }

        fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Err(InvalidCast)
        }

        fn serialize_tuple_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Err(InvalidCast)
        }

        fn serialize_tuple_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Err(InvalidCast)
        }

        fn serialize_map(
            self,
            _: core::option::Option<usize>,
        ) -> Result<Self::SerializeMap, Self::Error> {
            Err(InvalidCast)
        }

        fn serialize_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            Err(InvalidCast)
        }

        fn serialize_struct_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Err(InvalidCast)
        }
    }

    erased_serde::serialize(v, CastSerializer(Default::default()))
        .unwrap_or(Cast::Primitive(Primitive::None))
}

impl Error {
    fn serde() -> Self {
        Error::msg("`serde` serialization failed")
    }
}

#[derive(Debug)]
struct InvalidCast;

impl fmt::Display for InvalidCast {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid cast")
    }
}

impl serde_lib::ser::Error for InvalidCast {
    fn custom<T>(_: T) -> Self
    where
        T: fmt::Display,
    {
        InvalidCast
    }
}

impl serde_lib::ser::StdError for InvalidCast {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::Token;

    #[test]
    fn serde_capture() {
        assert_eq!(ValueBag::capture_serde(&42u64).to_token(), Token::U64(42));
    }

    #[test]
    fn serde_cast() {
        assert_eq!(
            42u32,
            ValueBag::capture_serde(&42u64)
                .to_u32()
                .expect("invalid value")
        );

        assert_eq!(
            "a string",
            ValueBag::capture_serde(&"a string")
                .to_borrowed_str()
                .expect("invalid value")
        );

        #[cfg(feature = "std")]
        assert_eq!(
            "a string",
            ValueBag::capture_serde(&"a string")
                .to_str()
                .expect("invalid value")
        );
    }

    #[test]
    fn serde_downcast() {
        #[derive(Debug, PartialEq, Eq)]
        struct Timestamp(usize);

        impl serde_lib::Serialize for Timestamp {
            fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
            where
                S: serde_lib::Serializer,
            {
                s.serialize_u64(self.0 as u64)
            }
        }

        let ts = Timestamp(42);

        assert_eq!(
            &ts,
            ValueBag::capture_serde(&ts)
                .downcast_ref::<Timestamp>()
                .expect("invalid value")
        );
    }

    #[test]
    fn serde_structured() {
        use serde_test::{assert_ser_tokens, Token};

        assert_ser_tokens(&ValueBag::from(42u64), &[Token::U64(42)]);
    }

    #[test]
    fn serde_debug() {
        struct TestSerde;

        impl serde_lib::Serialize for TestSerde {
            fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
            where
                S: serde_lib::Serializer,
            {
                s.serialize_u64(42)
            }
        }

        assert_eq!(
            format!("{:04?}", 42u64),
            format!("{:04?}", ValueBag::capture_serde(&TestSerde)),
        );
    }

    #[test]
    #[cfg(feature = "sval")]
    fn serde_sval() {
        use sval::test::Token;

        struct TestSerde;

        impl serde_lib::Serialize for TestSerde {
            fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
            where
                S: serde_lib::Serializer,
            {
                s.serialize_u64(42)
            }
        }

        assert_eq!(
            vec![Token::Unsigned(42)],
            sval::test::tokens(ValueBag::capture_serde(&TestSerde))
        );
    }

    #[cfg(feature = "std")]
    mod std_support {
        use super::*;

        use crate::std::borrow::ToOwned;

        #[test]
        fn serde_cast() {
            assert_eq!(
                "a string",
                ValueBag::capture_serde(&"a string".to_owned())
                    .to_str()
                    .expect("invalid value")
            );
        }
    }
}
