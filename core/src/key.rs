use core::{borrow::Borrow, fmt, hash, marker::PhantomData};

pub struct Key<'k> {
    value: *const str,
    value_static: Option<&'static str>,
    #[cfg(feature = "alloc")]
    value_owned: Option<Box<str>>,
    _marker: PhantomData<&'k str>,
}

impl<'k> fmt::Debug for Key<'k> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<'k> fmt::Display for Key<'k> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

unsafe impl<'k> Send for Key<'k> {}
unsafe impl<'k> Sync for Key<'k> {}

impl<'k> Clone for Key<'k> {
    fn clone(&self) -> Self {
        match self {
            #[cfg(feature = "alloc")]
            Key {
                value_static,
                value_owned,
                value,
                _marker,
            } => match value_owned {
                Some(value_owned) => {
                    let value_owned = value_owned.clone();

                    Key {
                        value: &*value_owned as *const str,
                        value_owned: Some(value_owned),
                        value_static: None,
                        _marker: PhantomData,
                    }
                }
                None => Key {
                    value: *value,
                    value_static: *value_static,
                    value_owned: None,
                    _marker: PhantomData,
                },
            },
            #[cfg(not(feature = "alloc"))]
            Key {
                value,
                value_static,
                _marker,
            } => Key {
                value: *value,
                value_static: *value_static,
                _marker: PhantomData,
            },
        }
    }
}

impl Key<'static> {
    pub const fn new(k: &'static str) -> Self {
        Key {
            value: k as *const str,
            value_static: Some(k),
            #[cfg(feature = "alloc")]
            value_owned: None,
            _marker: PhantomData,
        }
    }
}

impl<'k> Key<'k> {
    pub const fn new_ref(k: &'k str) -> Key<'k> {
        Key {
            value: k as *const str,
            value_static: None,
            #[cfg(feature = "alloc")]
            value_owned: None,
            _marker: PhantomData,
        }
    }

    pub fn by_ref<'b>(&'b self) -> Key<'b> {
        Key {
            value: self.value,
            value_static: self.value_static,
            #[cfg(feature = "alloc")]
            value_owned: None,
            _marker: PhantomData,
        }
    }

    pub fn as_str(&self) -> &str {
        unsafe { &(*self.value) }
    }

    pub fn as_static_str(&self) -> Option<&'static str> {
        self.value_static
    }
}

impl<'a> hash::Hash for Key<'a> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl<'a, 'b> PartialEq<Key<'b>> for Key<'a> {
    fn eq(&self, other: &Key<'b>) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> Eq for Key<'a> {}

impl<'a> PartialEq<str> for Key<'a> {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl<'a> PartialEq<Key<'a>> for str {
    fn eq(&self, other: &Key<'a>) -> bool {
        self == other.as_str()
    }
}

impl<'a, 'b> PartialEq<&'b str> for Key<'a> {
    fn eq(&self, other: &&'b str) -> bool {
        self.as_str() == *other
    }
}

impl<'a, 'b> PartialEq<Key<'b>> for &'a str {
    fn eq(&self, other: &Key<'b>) -> bool {
        *self == other.as_str()
    }
}

impl<'a, 'b> PartialOrd<Key<'b>> for Key<'a> {
    fn partial_cmp(&self, other: &Key<'b>) -> Option<core::cmp::Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl<'a> Ord for Key<'a> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl<'k> Borrow<str> for Key<'k> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<'k> AsRef<str> for Key<'k> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'a> From<&'a str> for Key<'a> {
    fn from(value: &'a str) -> Self {
        Key::new_ref(value)
    }
}

impl<'k> ToValue for Key<'k> {
    fn to_value(&self) -> Value {
        Value::from(self.as_str())
    }
}

impl<'v> Value<'v> {
    pub fn to_key(&self) -> Option<Key<'v>> {
        #[cfg(feature = "alloc")]
        {
            self.to_str().map(Key::new_cow_ref)
        }
        #[cfg(not(feature = "alloc"))]
        {
            self.to_borrowed_str().map(Key::new_ref)
        }
    }
}

pub trait ToKey {
    fn to_key(&self) -> Key;
}

impl<'a, T: ToKey + ?Sized> ToKey for &'a T {
    fn to_key(&self) -> Key {
        (**self).to_key()
    }
}

impl<'k> ToKey for Key<'k> {
    fn to_key(&self) -> Key {
        self.by_ref()
    }
}

impl ToKey for str {
    fn to_key(&self) -> Key {
        Key::new_ref(self)
    }
}

#[cfg(feature = "sval")]
impl<'k> sval::Value for Key<'k> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        use sval_ref::ValueRef as _;

        self.stream_ref(stream)
    }
}

#[cfg(feature = "sval")]
impl<'k> sval_ref::ValueRef<'k> for Key<'k> {
    fn stream_ref<S: sval::Stream<'k> + ?Sized>(&self, stream: &mut S) -> sval::Result {
        if let Some(k) = self.as_static_str() {
            stream.value(k)
        } else {
            stream.value_computed(self.as_str())
        }
    }
}

#[cfg(feature = "serde")]
impl<'k> serde::Serialize for Key<'k> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_str().serialize(serializer)
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use alloc::borrow::Cow;

    use super::*;

    impl Key<'static> {
        pub fn new_owned(key: impl Into<Box<str>>) -> Self {
            let value = key.into();

            Key {
                value: &*value as *const str,
                value_static: None,
                value_owned: Some(value),
                _marker: PhantomData,
            }
        }
    }

    impl<'k> Key<'k> {
        pub fn new_cow_ref(key: Cow<'k, str>) -> Self {
            match key {
                Cow::Borrowed(key) => Key::new_ref(key),
                Cow::Owned(key) => Key::new_owned(key),
            }
        }

        pub fn to_cow(&self) -> Cow<'static, str> {
            match self.value_static {
                Some(key) => Cow::Borrowed(key),
                None => Cow::Owned(self.as_str().to_owned()),
            }
        }

        pub fn to_owned(&self) -> Key<'static> {
            match self.value_static {
                Some(key) => Key::new(key),
                None => Key::new_owned(self.as_str()),
            }
        }
    }
}

use crate::value::{ToValue, Value};

#[cfg(feature = "alloc")]
pub use self::alloc_support::*;
