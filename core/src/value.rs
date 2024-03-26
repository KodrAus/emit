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

    pub fn as_f64(&self) -> f64 {
        self.0.as_f64()
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

macro_rules! impl_primitive {
    ($($t:ty,)*) => {
        $(
            impl ToValue for $t {
                fn to_value(&self) -> Value {
                    Value(self.into())
                }
            }

            impl<const N: usize> ToValue for [$t; N] {
                fn to_value(&self) -> Value {
                    Value(self.into())
                }
            }

            impl<'v> FromValue<'v> for $t {
                fn from_value(value: Value<'v>) -> Option<Self> {
                    value.0.try_into().ok()
                }
            }

            impl<'v> From<$t> for Value<'v> {
                fn from(value: $t) -> Self {
                    Value(value.into())
                }
            }

            impl<'v> From<Option<$t>> for Value<'v> {
                fn from(value: Option<$t>) -> Self {
                    Value(value_bag::ValueBag::from_option(value))
                }
            }
        )*
    };
}

macro_rules! impl_ref {
    ($(& $l:lifetime $t:ty,)*) => {
        $(
            impl ToValue for $t {
                fn to_value(&self) -> Value {
                    Value(self.into())
                }
            }

            impl<$l, const N: usize> ToValue for [&$l $t; N] {
                fn to_value(&self) -> Value {
                    Value(self.into())
                }
            }

            impl<$l> FromValue<$l> for &$l $t {
                fn from_value(value: Value<$l>) -> Option<Self> {
                    value.0.try_into().ok()
                }
            }

            impl<$l> From<&$l $t> for Value<$l> {
                fn from(value: &$l $t) -> Self {
                    Value(value.into())
                }
            }

            impl<$l> From<Option<&$l $t>> for Value<$l> {
                fn from(value: Option<&$l $t>) -> Self {
                    Value(value_bag::ValueBag::from_option(value))
                }
            }
        )*
    };
}

impl_primitive!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f64, bool,);

impl_ref!(&'v str,);

impl ToValue for dyn fmt::Debug {
    fn to_value(&self) -> Value {
        Value(value_bag::ValueBag::from_dyn_debug(self))
    }
}

impl ToValue for dyn fmt::Display {
    fn to_value(&self) -> Value {
        Value(value_bag::ValueBag::from_dyn_display(self))
    }
}

#[cfg(feature = "std")]
impl ToValue for (dyn std::error::Error + 'static) {
    fn to_value(&self) -> Value {
        Value(value_bag::ValueBag::from_dyn_error(self))
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
