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

    pub fn capture_debug(value: &'v (impl fmt::Debug + 'static)) -> Self {
        Value(value_bag::ValueBag::capture_debug(value))
    }

    pub fn from_debug(value: &'v impl fmt::Debug) -> Self {
        Value(value_bag::ValueBag::from_debug(value))
    }

    #[cfg(feature = "serde")]
    pub fn capture_serde(value: &'v (impl serde::Serialize + 'static)) -> Self {
        Value(value_bag::ValueBag::capture_serde1(value))
    }

    #[cfg(feature = "serde")]
    pub fn from_serde(value: &'v impl serde::Serialize) -> Self {
        Value(value_bag::ValueBag::from_serde1(value))
    }

    #[cfg(feature = "sval")]
    pub fn capture_sval(value: &'v (impl sval::Value + 'static)) -> Self {
        Value(value_bag::ValueBag::capture_sval2(value))
    }

    #[cfg(feature = "sval")]
    pub fn from_sval(value: &'v impl sval::Value) -> Self {
        Value(value_bag::ValueBag::from_sval2(value))
    }

    #[cfg(feature = "std")]
    pub fn capture_error(value: &'v (impl std::error::Error + 'static)) -> Self {
        Value(value_bag::ValueBag::capture_error(value))
    }

    pub fn from_any(value: &'v impl ToValue) -> Self {
        value.to_value()
    }

    pub fn null() -> Self {
        Value(value_bag::ValueBag::empty())
    }

    pub fn by_ref<'b>(&'b self) -> Value<'b> {
        Value(self.0.by_ref())
    }

    pub fn cast<'a, T: FromValue<'v>>(self) -> Option<T> {
        T::from_value(self)
    }

    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }

    pub fn parse<T: FromStr>(&self) -> Option<T> {
        struct Extract<T>(Option<T>);

        impl<'v, T: FromStr> value_bag::visit::Visit<'v> for Extract<T> {
            fn visit_any(&mut self, value: value_bag::ValueBag) -> Result<(), value_bag::Error> {
                #[cfg(feature = "alloc")]
                {
                    self.0 = value.to_string().parse().ok();

                    Ok(())
                }
                #[cfg(not(feature = "alloc"))]
                {
                    let _ = value;

                    Ok(())
                }
            }

            fn visit_str(&mut self, value: &str) -> Result<(), value_bag::Error> {
                self.0 = value.parse().ok();

                Ok(())
            }
        }

        let mut visitor = Extract(None);
        let _ = self.0.visit(&mut visitor);
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

    pub fn to_usize(&self) -> Option<usize> {
        self.0.to_u64()?.try_into().ok()
    }

    pub fn to_i64(&self) -> Option<i64> {
        self.0.to_i64()
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

pub trait FromValue<'v> {
    fn from_value(value: Value<'v>) -> Option<Self>
    where
        Self: Sized;
}

impl<'a, T: ToValue + ?Sized> ToValue for &'a T {
    fn to_value(&self) -> Value {
        (**self).to_value()
    }
}

impl<T: ToValue> ToValue for Option<T> {
    fn to_value(&self) -> Value {
        match self {
            Some(v) => v.to_value(),
            None => Value::null(),
        }
    }
}

impl<'v> ToValue for Value<'v> {
    fn to_value(&self) -> Value {
        self.by_ref()
    }
}

impl<'v> FromValue<'v> for Value<'v> {
    fn from_value(value: Value<'v>) -> Option<Self> {
        Some(value)
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

impl<'v> FromValue<'v> for &'v str {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value.to_borrowed_str()
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

impl<'v> FromValue<'v> for usize {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value.to_usize()
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

impl<'v> FromValue<'v> for f64 {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value.to_f64()
    }
}

impl<const N: usize> ToValue for [f64; N] {
    fn to_value(&self) -> Value {
        Value::from(self)
    }
}

impl<'v, const N: usize> From<&'v [f64; N]> for Value<'v> {
    fn from(value: &'v [f64; N]) -> Self {
        Value(value.into())
    }
}

impl ToValue for i64 {
    fn to_value(&self) -> Value {
        Value::from(*self)
    }
}

impl<'v> From<i64> for Value<'v> {
    fn from(value: i64) -> Self {
        Value(value.into())
    }
}

impl<'v> FromValue<'v> for i64 {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value.to_i64()
    }
}

impl<const N: usize> ToValue for [i64; N] {
    fn to_value(&self) -> Value {
        Value::from(self)
    }
}

impl<'v, const N: usize> From<&'v [i64; N]> for Value<'v> {
    fn from(value: &'v [i64; N]) -> Self {
        Value(value.into())
    }
}

impl ToValue for dyn fmt::Debug {
    fn to_value(&self) -> Value {
        Value(value_bag::ValueBag::from_dyn_debug(self))
    }
}

impl<'v> From<&'v dyn fmt::Debug> for Value<'v> {
    fn from(value: &'v dyn fmt::Debug) -> Self {
        Value(value_bag::ValueBag::from_dyn_debug(value))
    }
}

impl ToValue for dyn fmt::Display {
    fn to_value(&self) -> Value {
        Value(value_bag::ValueBag::from_dyn_display(self))
    }
}

impl<'v> From<&'v dyn fmt::Display> for Value<'v> {
    fn from(value: &'v dyn fmt::Display) -> Self {
        Value(value_bag::ValueBag::from_dyn_display(value))
    }
}

#[cfg(feature = "std")]
impl ToValue for (dyn std::error::Error + 'static) {
    fn to_value(&self) -> Value {
        Value(value_bag::ValueBag::from_dyn_error(self))
    }
}

#[cfg(feature = "std")]
impl<'v> From<&'v (dyn std::error::Error + 'static)> for Value<'v> {
    fn from(value: &'v (dyn std::error::Error + 'static)) -> Self {
        Value(value_bag::ValueBag::from_dyn_error(value))
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    use alloc::borrow::Cow;

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

        pub fn to_cow_str(&self) -> Option<Cow<'v, str>> {
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

    impl ToValue for OwnedValue {
        fn to_value(&self) -> Value {
            self.by_ref()
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
