use core::{borrow::Borrow, fmt, hash, marker::PhantomData};

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, sync::Arc};

pub struct Str<'k> {
    // This type is an optimized `Cow<str>`
    // It avoids the cost of matching the variant to get the inner value
    value: *const str,
    // Only one of `value_static`, `value_owned`, or `value_shared` will be set
    value_static: Option<&'static str>,
    #[cfg(feature = "alloc")]
    value_owned: Option<Box<str>>,
    #[cfg(feature = "alloc")]
    value_shared: Option<Arc<str>>,
    _marker: PhantomData<&'k str>,
}

impl<'k> fmt::Debug for Str<'k> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.get(), f)
    }
}

impl<'k> fmt::Display for Str<'k> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.get(), f)
    }
}

unsafe impl<'k> Send for Str<'k> {}
unsafe impl<'k> Sync for Str<'k> {}

impl<'k> Clone for Str<'k> {
    fn clone(&self) -> Self {
        #[cfg(feature = "alloc")]
        {
            match self {
                Str {
                    value: _,
                    value_static: _,
                    value_owned: Some(value_owned),
                    value_shared: _,
                    _marker,
                } => {
                    let value_owned = value_owned.clone();

                    Str {
                        value: &*value_owned as *const str,
                        value_owned: Some(value_owned),
                        value_shared: None,
                        value_static: None,
                        _marker: PhantomData,
                    }
                }
                Str {
                    value: _,
                    value_static: _,
                    value_owned: _,
                    value_shared: Some(value_shared),
                    _marker,
                } => {
                    let value_shared = value_shared.clone();

                    Str {
                        value: &*value_shared as *const str,
                        value_shared: Some(value_shared),
                        value_owned: None,
                        value_static: None,
                        _marker: PhantomData,
                    }
                }
                Str {
                    value,
                    value_static,
                    value_owned: _,
                    value_shared: _,
                    _marker,
                } => Str {
                    value: *value,
                    value_static: *value_static,
                    value_owned: None,
                    value_shared: None,
                    _marker: PhantomData,
                },
            }
        }
        #[cfg(not(feature = "alloc"))]
        {
            Str {
                value: self.value,
                value_static: self.value_static,
                _marker: PhantomData,
            }
        }
    }
}

impl Str<'static> {
    pub const fn new(k: &'static str) -> Self {
        Str {
            value: k as *const str,
            value_static: Some(k),
            #[cfg(feature = "alloc")]
            value_owned: None,
            #[cfg(feature = "alloc")]
            value_shared: None,
            _marker: PhantomData,
        }
    }
}

impl<'k> Str<'k> {
    pub const fn new_ref(k: &'k str) -> Str<'k> {
        Str {
            value: k as *const str,
            value_static: None,
            #[cfg(feature = "alloc")]
            value_owned: None,
            #[cfg(feature = "alloc")]
            value_shared: None,
            _marker: PhantomData,
        }
    }

    pub const fn by_ref<'b>(&'b self) -> Str<'b> {
        Str {
            value: self.value,
            value_static: self.value_static,
            #[cfg(feature = "alloc")]
            value_owned: None,
            #[cfg(feature = "alloc")]
            value_shared: None,
            _marker: PhantomData,
        }
    }

    pub const fn get(&self) -> &str {
        // NOTE: It's important here that the lifetime returned is not `'k`
        // If it was it would be possible to return a `&'static str` from
        // an owned value
        unsafe { &(*self.value) }
    }

    pub const fn get_static(&self) -> Option<&'static str> {
        self.value_static
    }
}

impl<'a> hash::Hash for Str<'a> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state)
    }
}

impl<'a, 'b> PartialEq<Str<'b>> for Str<'a> {
    fn eq(&self, other: &Str<'b>) -> bool {
        self.get() == other.get()
    }
}

impl<'a> Eq for Str<'a> {}

impl<'a> PartialEq<str> for Str<'a> {
    fn eq(&self, other: &str) -> bool {
        self.get() == other
    }
}

impl<'a> PartialEq<Str<'a>> for str {
    fn eq(&self, other: &Str<'a>) -> bool {
        self == other.get()
    }
}

impl<'a, 'b> PartialEq<&'b str> for Str<'a> {
    fn eq(&self, other: &&'b str) -> bool {
        self.get() == *other
    }
}

impl<'a, 'b> PartialEq<Str<'b>> for &'a str {
    fn eq(&self, other: &Str<'b>) -> bool {
        *self == other.get()
    }
}

impl<'a, 'b> PartialOrd<Str<'b>> for Str<'a> {
    fn partial_cmp(&self, other: &Str<'b>) -> Option<core::cmp::Ordering> {
        self.get().partial_cmp(other.get())
    }
}

impl<'a> Ord for Str<'a> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.get().cmp(other.get())
    }
}

impl<'k> Borrow<str> for Str<'k> {
    fn borrow(&self) -> &str {
        self.get()
    }
}

impl<'k> AsRef<str> for Str<'k> {
    fn as_ref(&self) -> &str {
        self.get()
    }
}

impl<'a> From<&'a str> for Str<'a> {
    fn from(value: &'a str) -> Self {
        Str::new_ref(value)
    }
}

impl<'a, 'b> From<&'a Str<'b>> for Str<'a> {
    fn from(value: &'a Str<'b>) -> Self {
        value.by_ref()
    }
}

impl<'k> ToValue for Str<'k> {
    fn to_value(&self) -> Value {
        self.get().to_value()
    }
}

impl<'k> FromValue<'k> for Str<'k> {
    fn from_value<'a>(value: Value<'k>) -> Option<Self> {
        #[cfg(feature = "alloc")]
        {
            value.to_cow_str().map(Str::new_cow_ref)
        }
        #[cfg(not(feature = "alloc"))]
        {
            value.to_borrowed_str().map(Str::new_ref)
        }
    }
}

pub trait ToStr {
    fn to_str(&self) -> Str;
}

impl<'a, T: ToStr + ?Sized> ToStr for &'a T {
    fn to_str(&self) -> Str {
        (**self).to_str()
    }
}

impl<'k> ToStr for Str<'k> {
    fn to_str(&self) -> Str {
        self.by_ref()
    }
}

impl ToStr for str {
    fn to_str(&self) -> Str {
        Str::new_ref(self)
    }
}

#[cfg(feature = "sval")]
impl<'k> sval::Value for Str<'k> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        use sval_ref::ValueRef as _;

        self.stream_ref(stream)
    }
}

#[cfg(feature = "sval")]
impl<'k> sval_ref::ValueRef<'k> for Str<'k> {
    fn stream_ref<S: sval::Stream<'k> + ?Sized>(&self, stream: &mut S) -> sval::Result {
        if let Some(k) = self.get_static() {
            stream.value(k)
        } else {
            stream.value_computed(self.get())
        }
    }
}

#[cfg(feature = "serde")]
impl<'k> serde::Serialize for Str<'k> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.get().serialize(serializer)
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use alloc::borrow::Cow;

    use super::*;

    impl Str<'static> {
        pub fn new_owned(key: impl Into<Box<str>>) -> Self {
            let value = key.into();

            Str {
                value: &*value as *const str,
                value_owned: Some(value),
                value_shared: None,
                value_static: None,
                _marker: PhantomData,
            }
        }

        pub fn new_shared(key: impl Into<Arc<str>>) -> Self {
            let value = key.into();

            Str {
                value: &*value as *const str,
                value_shared: Some(value),
                value_owned: None,
                value_static: None,
                _marker: PhantomData,
            }
        }
    }

    impl<'k> Str<'k> {
        pub fn new_cow_ref(key: Cow<'k, str>) -> Self {
            match key {
                Cow::Borrowed(key) => Str::new_ref(key),
                Cow::Owned(key) => Str::new_owned(key),
            }
        }

        pub fn to_cow(&self) -> Cow<'static, str> {
            match self.value_static {
                Some(key) => Cow::Borrowed(key),
                None => Cow::Owned(self.get().to_owned()),
            }
        }

        pub fn to_owned(&self) -> Str<'static> {
            match self {
                Str {
                    value: _,
                    value_static: Some(value_static),
                    value_owned: _,
                    value_shared: _,
                    _marker,
                } => Str::new(value_static),
                Str {
                    value: _,
                    value_static: _,
                    value_owned: Some(value_owned),
                    value_shared: _,
                    _marker,
                } => Str::new_owned(value_owned.clone()),
                Str {
                    value: _,
                    value_static: _,
                    value_owned: _,
                    value_shared: Some(value_shared),
                    _marker,
                } => Str::new_shared(value_shared.clone()),
                str => Str::new_owned(str.get()),
            }
        }

        pub fn to_shared(&self) -> Str<'static> {
            match self {
                Str {
                    value: _,
                    value_static: Some(value_static),
                    value_owned: _,
                    value_shared: _,
                    _marker,
                } => Str::new(value_static),
                Str {
                    value: _,
                    value_static: _,
                    value_owned: Some(value_owned),
                    value_shared: _,
                    _marker,
                } => Str::new_shared(value_owned.clone()),
                Str {
                    value: _,
                    value_static: _,
                    value_owned: _,
                    value_shared: Some(value_shared),
                    _marker,
                } => Str::new_shared(value_shared.clone()),
                str => Str::new_shared(str.get()),
            }
        }
    }

    impl ToStr for String {
        fn to_str(&self) -> Str {
            Str::new_ref(self)
        }
    }

    impl From<String> for Str<'static> {
        fn from(value: String) -> Self {
            Str::new_owned(value)
        }
    }

    impl From<Box<str>> for Str<'static> {
        fn from(value: Box<str>) -> Self {
            Str::new_owned(value)
        }
    }

    impl From<Arc<str>> for Str<'static> {
        fn from(value: Arc<str>) -> Self {
            Str::new_shared(value)
        }
    }

    impl<'k> From<&'k String> for Str<'k> {
        fn from(value: &'k String) -> Self {
            Str::new_ref(value)
        }
    }
}

use crate::value::{FromValue, ToValue, Value};
