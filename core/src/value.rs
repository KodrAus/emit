use core::{fmt, str::FromStr};

#[derive(Clone)]
pub struct Value<'v>(value_bag::ValueBag<'v>);

impl<'v> Value<'v> {
    pub fn capture_display(value: &'v (impl fmt::Display + 'static)) -> Self {
        Value(value_bag::ValueBag::capture_display(value))
    }

    pub fn from_display(value: &'v impl fmt::Display) -> Self {
        Value(value_bag::ValueBag::from_display(value))
    }

    pub fn by_ref<'b>(&'b self) -> Value<'b> {
        Value(self.0.by_ref())
    }

    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }

    pub fn visit(&self, visitor: impl Visitor<'v>) {
        struct Visit<V>(V);

        impl<'v, V: Visitor<'v>> value_bag::visit::Visit<'v> for Visit<V> {
            fn visit_any(&mut self, value: value_bag::ValueBag) -> Result<(), value_bag::Error> {
                self.0.visit_any(Value(value));

                Ok(())
            }

            fn visit_str(&mut self, value: &str) -> Result<(), value_bag::Error> {
                self.0.visit_str(value);

                Ok(())
            }
        }

        let _ = self.0.visit(Visit(visitor));
    }

    pub fn parse<T: FromStr>(&self) -> Option<T> {
        struct Extract<T>(Option<T>);

        impl<'v, T: FromStr> Visitor<'v> for Extract<T> {
            fn visit_any(&mut self, _: Value) {}

            fn visit_str(&mut self, value: &str) {
                self.0 = value.parse().ok();
            }
        }

        let mut visitor = Extract(None);
        self.visit(&mut visitor);
        visitor.0
    }

    pub fn to_borrowed_str(&self) -> Option<&'v str> {
        self.0.to_borrowed_str()
    }

    pub fn to_f64(&self) -> Option<f64> {
        self.0.to_f64()
    }
}

pub trait Visitor<'v> {
    fn visit_any(&mut self, value: Value);

    fn visit_str(&mut self, value: &str) {
        self.visit_any(Value::from(value))
    }

    fn visit_f64(&mut self, value: f64) {
        self.visit_any(Value::from(value))
    }
}

impl<'a, 'v, V: Visitor<'v> + ?Sized> Visitor<'v> for &'a mut V {
    fn visit_any(&mut self, value: Value) {
        (**self).visit_any(value)
    }

    fn visit_str(&mut self, value: &str) {
        (**self).visit_str(value)
    }

    fn visit_f64(&mut self, value: f64) {
        (**self).visit_f64(value)
    }
}

impl<'v> fmt::Debug for Value<'v> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl<'v> fmt::Display for Value<'v> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(feature = "sval")]
impl<'v> sval::Value for Value<'v> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        self.0.stream(stream)
    }
}

#[cfg(feature = "sval")]
impl<'v> sval_ref::ValueRef<'v> for Value<'v> {
    fn stream_ref<S: sval::Stream<'v> + ?Sized>(&self, stream: &mut S) -> sval::Result {
        self.0.stream_ref(stream)
    }
}

#[cfg(feature = "serde")]
impl<'v> serde::Serialize for Value<'v> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

pub trait ToValue {
    fn to_value(&self) -> Value;
}

impl<'a, T: ToValue + ?Sized> ToValue for &'a T {
    fn to_value(&self) -> Value {
        (**self).to_value()
    }
}

impl<'v> ToValue for Value<'v> {
    fn to_value(&self) -> Value {
        self.by_ref()
    }
}

impl<'v> ToValue for value_bag::ValueBag<'v> {
    fn to_value(&self) -> Value {
        Value(self.by_ref())
    }
}

impl<'v> From<value_bag::ValueBag<'v>> for Value<'v> {
    fn from(value: value_bag::ValueBag<'v>) -> Self {
        Value(value)
    }
}

impl ToValue for str {
    fn to_value(&self) -> Value {
        Value::from(self)
    }
}

impl<'v> From<&'v str> for Value<'v> {
    fn from(value: &'v str) -> Self {
        Value(value.into())
    }
}

impl ToValue for usize {
    fn to_value(&self) -> Value {
        Value::from(*self)
    }
}

impl<'v> From<usize> for Value<'v> {
    fn from(value: usize) -> Self {
        Value(value.into())
    }
}

impl ToValue for f64 {
    fn to_value(&self) -> Value {
        Value::from(*self)
    }
}

impl<'v> From<f64> for Value<'v> {
    fn from(value: f64) -> Self {
        Value(value.into())
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    use alloc::{borrow::Cow, vec::Vec};

    impl<'v> Value<'v> {
        #[cfg(any(feature = "sval", feature = "serde"))]
        pub fn to_f64_sequence(&self) -> Option<Vec<f64>> {
            #[derive(Default)]
            struct F64Vec(Vec<f64>);

            impl<'a> Extend<Option<Value<'a>>> for F64Vec {
                fn extend<T: IntoIterator<Item = Option<Value<'a>>>>(&mut self, iter: T) {
                    for v in iter {
                        self.0.push(v.and_then(|v| v.to_f64()).unwrap_or(f64::NAN));
                    }
                }
            }

            self.to_sequence::<F64Vec>().map(|seq| seq.0)
        }
    }

    #[derive(Clone)]
    pub struct OwnedValue(value_bag::OwnedValueBag);

    impl<'v> Value<'v> {
        pub fn to_owned(&self) -> OwnedValue {
            OwnedValue(self.0.to_owned())
        }

        pub fn to_str(&self) -> Option<Cow<'v, str>> {
            self.0.to_str()
        }
    }

    impl OwnedValue {
        pub fn by_ref<'v>(&'v self) -> Value<'v> {
            Value(self.0.by_ref())
        }
    }

    impl fmt::Debug for OwnedValue {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Debug::fmt(&self.0, f)
        }
    }

    impl fmt::Display for OwnedValue {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(&self.0, f)
        }
    }

    impl<'a> From<&'a OwnedValue> for Value<'a> {
        fn from(value: &'a OwnedValue) -> Self {
            value.by_ref()
        }
    }

    impl From<value_bag::OwnedValueBag> for OwnedValue {
        fn from(value: value_bag::OwnedValueBag) -> Self {
            OwnedValue(value)
        }
    }

    impl ToValue for OwnedValue {
        fn to_value(&self) -> Value {
            self.by_ref()
        }
    }

    impl ToValue for value_bag::OwnedValueBag {
        fn to_value(&self) -> Value {
            Value(self.by_ref())
        }
    }

    #[cfg(feature = "sval")]
    impl sval::Value for OwnedValue {
        fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(
            &'sval self,
            stream: &mut S,
        ) -> sval::Result {
            self.0.stream(stream)
        }
    }

    #[cfg(feature = "serde")]
    impl serde::Serialize for OwnedValue {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.0.serialize(serializer)
        }
    }
}

#[cfg(feature = "alloc")]
pub use self::alloc_support::*;

#[cfg(all(feature = "sval", not(feature = "serde")))]
mod seq {
    use super::*;

    impl<'v> Value<'v> {
        pub(super) fn to_sequence<E: Default + for<'a> Extend<Option<Value<'a>>>>(
            &self,
        ) -> Option<E> {
            if let Ok(seq) = sval_nested::stream_ref(Root(Default::default()), &self.0) {
                Some(seq)
            } else {
                None
            }
        }
    }

    struct Root<C>(C);

    struct Seq<C>(C);

    impl<'sval, C: Default + for<'a> Extend<Option<Value<'a>>>> sval_nested::Stream<'sval> for Root<C> {
        type Ok = C;

        type Seq = Seq<C>;

        type Map = sval_nested::Unsupported<C>;

        type Tuple = sval_nested::Unsupported<C>;

        type Record = sval_nested::Unsupported<C>;

        type Enum = sval_nested::Unsupported<C>;

        fn null(self) -> sval_nested::Result<Self::Ok> {
            Err(sval_nested::Error::invalid_value("not a sequence"))
        }

        fn bool(self, _: bool) -> sval_nested::Result<Self::Ok> {
            Err(sval_nested::Error::invalid_value("not a sequence"))
        }

        fn i64(self, _: i64) -> sval_nested::Result<Self::Ok> {
            Err(sval_nested::Error::invalid_value("not a sequence"))
        }

        fn f64(self, _: f64) -> sval_nested::Result<Self::Ok> {
            Err(sval_nested::Error::invalid_value("not a sequence"))
        }

        fn text_computed(self, _: &str) -> sval_nested::Result<Self::Ok> {
            Err(sval_nested::Error::invalid_value("not a sequence"))
        }

        fn seq_begin(self, _: Option<usize>) -> sval_nested::Result<Self::Seq> {
            Ok(Seq(C::default()))
        }

        fn map_begin(self, _: Option<usize>) -> sval_nested::Result<Self::Map> {
            Err(sval_nested::Error::invalid_value("not a sequence"))
        }

        fn tuple_begin(
            self,
            _: Option<sval::Tag>,
            _: Option<sval::Label>,
            _: Option<sval::Index>,
            _: Option<usize>,
        ) -> sval_nested::Result<Self::Tuple> {
            Err(sval_nested::Error::invalid_value("not a sequence"))
        }

        fn record_begin(
            self,
            _: Option<sval::Tag>,
            _: Option<sval::Label>,
            _: Option<sval::Index>,
            _: Option<usize>,
        ) -> sval_nested::Result<Self::Record> {
            Err(sval_nested::Error::invalid_value("not a sequence"))
        }

        fn enum_begin(
            self,
            _: Option<sval::Tag>,
            _: Option<sval::Label>,
            _: Option<sval::Index>,
        ) -> sval_nested::Result<Self::Enum> {
            Err(sval_nested::Error::invalid_value("not a sequence"))
        }
    }

    impl<'sval, C: for<'a> Extend<Option<Value<'a>>>> sval_nested::StreamSeq<'sval> for Seq<C> {
        type Ok = C;

        fn value_computed<V: sval::Value>(&mut self, value: V) -> sval_nested::Result {
            self.0
                .extend(Some(Some(Value(value_bag::ValueBag::from_sval2(&value)))));
            Ok(())
        }

        fn end(self) -> sval_nested::Result<Self::Ok> {
            Ok(self.0)
        }
    }
}

#[cfg(feature = "serde")]
mod seq {
    use super::*;

    impl<'v> Value<'v> {
        pub(super) fn to_sequence<E: Default + for<'a> Extend<Option<Value<'a>>>>(
            &self,
        ) -> Option<E> {
            if let Ok(seq) = serde::Serialize::serialize(&self.0, Root(Default::default())) {
                Some(seq)
            } else {
                None
            }
        }
    }

    struct Root<C>(C);

    struct Seq<C>(C);

    #[derive(Debug)]
    struct Unsupported;

    impl fmt::Display for Unsupported {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "not a sequence")
        }
    }

    impl serde::ser::Error for Unsupported {
        fn custom<T>(_: T) -> Self
        where
            T: fmt::Display,
        {
            Unsupported
        }
    }

    impl serde::ser::StdError for Unsupported {}

    impl<C: Default + for<'a> Extend<Option<Value<'a>>>> serde::Serializer for Root<C> {
        type Ok = C;

        type Error = Unsupported;

        type SerializeSeq = Seq<C>;

        type SerializeTuple = serde::ser::Impossible<C, Unsupported>;

        type SerializeTupleStruct = serde::ser::Impossible<C, Unsupported>;

        type SerializeTupleVariant = serde::ser::Impossible<C, Unsupported>;

        type SerializeMap = serde::ser::Impossible<C, Unsupported>;

        type SerializeStruct = serde::ser::Impossible<C, Unsupported>;

        type SerializeStructVariant = serde::ser::Impossible<C, Unsupported>;

        fn serialize_bool(self, _: bool) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_i8(self, _: i8) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_i16(self, _: i16) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_i32(self, _: i32) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_i64(self, _: i64) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_u8(self, _: u8) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_u16(self, _: u16) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_u32(self, _: u32) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_u64(self, _: u64) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_str(self, _: &str) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: serde::Serialize,
        {
            value.serialize(self)
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_unit_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_newtype_struct<T: ?Sized>(
            self,
            _: &'static str,
            _: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: serde::Serialize,
        {
            Err(Unsupported)
        }

        fn serialize_newtype_variant<T: ?Sized>(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: serde::Serialize,
        {
            Err(Unsupported)
        }

        fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Ok(Seq(C::default()))
        }

        fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_tuple_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_tuple_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            Err(Unsupported)
        }

        fn serialize_struct_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Err(Unsupported)
        }
    }

    impl<C: for<'a> Extend<Option<Value<'a>>>> serde::ser::SerializeSeq for Seq<C> {
        type Ok = C;

        type Error = Unsupported;

        fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: serde::Serialize,
        {
            self.0
                .extend(Some(Some(Value(value_bag::ValueBag::from_serde1(&value)))));
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(self.0)
        }
    }
}
