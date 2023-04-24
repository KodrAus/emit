use core::borrow::Borrow;

pub struct Key<'k> {
    value_ref: &'k str,
    value_static: Option<&'static str>,
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
