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

    pub fn as_f64(&self) -> f64 {
        self.0.as_f64()
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
        pub fn as_f64_sequence(&self) -> Vec<f64> {
            self.0.as_f64_seq()
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
