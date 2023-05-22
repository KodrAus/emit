use core::{borrow::Borrow, fmt, hash};

#[derive(Clone)]
pub struct Key<'k> {
    value_ref: &'k str,
    value_static: Option<&'static str>,
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

impl<'k> Key<'k> {
    pub fn new(k: &'static str) -> Key<'k> {
        Key {
            value_ref: k,
            value_static: Some(k),
        }
    }

    pub fn new_ref(k: &'k str) -> Key<'k> {
        Key {
            value_ref: k,
            value_static: None,
        }
    }

    pub fn by_ref<'b>(&'b self) -> Key<'b> {
        Key {
            value_ref: self.value_ref,
            value_static: self.value_static,
        }
    }

    pub fn as_str(&self) -> &str {
        self.value_ref
    }

    pub fn as_static_str(&self) -> Option<&'static str> {
        self.value_static
    }
}

impl<'a> From<&'a str> for Key<'a> {
    fn from(value: &'a str) -> Self {
        Key::new_ref(value)
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

#[cfg(feature = "alloc")]
mod alloc_support {
    use alloc::borrow::Cow;

    use super::*;

    #[derive(Clone)]
    pub struct OwnedKey(Cow<'static, str>);

    impl<'k> Key<'k> {
        pub fn to_owned(&self) -> OwnedKey {
            match self.value_static {
                Some(key) => OwnedKey(Cow::Borrowed(key)),
                None => OwnedKey(Cow::Owned(self.value_ref.to_owned())),
            }
        }
    }

    impl OwnedKey {
        pub fn by_ref<'k>(&'k self) -> Key<'k> {
            match self.0 {
                Cow::Borrowed(key) => Key::new(key),
                Cow::Owned(ref key) => Key::new_ref(key),
            }
        }

        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    impl<'k> From<Key<'k>> for OwnedKey {
        fn from(key: Key<'k>) -> Self {
            key.to_owned()
        }
    }

    impl<'k> From<&'k OwnedKey> for Key<'k> {
        fn from(key: &'k OwnedKey) -> Self {
            key.by_ref()
        }
    }

    impl hash::Hash for OwnedKey {
        fn hash<H: hash::Hasher>(&self, state: &mut H) {
            self.as_str().hash(state)
        }
    }

    impl PartialEq<OwnedKey> for OwnedKey {
        fn eq(&self, other: &OwnedKey) -> bool {
            self.as_str() == other.as_str()
        }
    }

    impl Eq for OwnedKey {}

    impl PartialEq<str> for OwnedKey {
        fn eq(&self, other: &str) -> bool {
            self.as_str() == other
        }
    }

    impl PartialEq<OwnedKey> for str {
        fn eq(&self, other: &OwnedKey) -> bool {
            self == other.as_str()
        }
    }

    impl<'a> PartialEq<&'a str> for OwnedKey {
        fn eq(&self, other: &&'a str) -> bool {
            self.as_str() == *other
        }
    }

    impl<'a> PartialEq<OwnedKey> for &'a str {
        fn eq(&self, other: &OwnedKey) -> bool {
            *self == other.as_str()
        }
    }

    impl<'a> PartialEq<Key<'a>> for OwnedKey {
        fn eq(&self, other: &Key<'a>) -> bool {
            self.as_str() == other
        }
    }

    impl<'a> PartialEq<OwnedKey> for Key<'a> {
        fn eq(&self, other: &OwnedKey) -> bool {
            self == other.as_str()
        }
    }

    impl PartialOrd<OwnedKey> for OwnedKey {
        fn partial_cmp(&self, other: &OwnedKey) -> Option<core::cmp::Ordering> {
            self.as_str().partial_cmp(other.as_str())
        }
    }

    impl Ord for OwnedKey {
        fn cmp(&self, other: &Self) -> core::cmp::Ordering {
            self.as_str().cmp(other.as_str())
        }
    }

    impl Borrow<str> for OwnedKey {
        fn borrow(&self) -> &str {
            self.as_str()
        }
    }

    impl AsRef<str> for OwnedKey {
        fn as_ref(&self) -> &str {
            self.as_str()
        }
    }
}

#[cfg(feature = "alloc")]
pub use self::alloc_support::*;
