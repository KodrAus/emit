use crate::std::fmt;

#[cfg(feature = "std")]
use crate::std::time::Duration;

pub struct RawEvent<'a> {
    pub timestamp: RawTimestamp,
    pub level: RawLevel,
    pub properties: &'a [(&'static str, ValueBag<'a>)],
    pub template: Template<'a>,
}

impl<'a> RawEvent<'a> {
    pub fn get(&self, key: impl AsRef<str>) -> Option<&ValueBag<'a>> {
        self.properties
            .binary_search_by_key(&key.as_ref(), |(k, _)| k)
            .ok()
            .map(|index| &self.properties[index].1)
    }
}

impl<'a> fmt::Display for RawEvent<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let rendered = self.template.render(fv_template::rt::Context::new().fill(
            move |write: &mut fmt::Formatter, label| {
                self.get(label)
                    .map(|value| fmt::Display::fmt(&value, write))
            },
        ));

        fmt::Display::fmt(&rendered, f)
    }
}

pub use fv_template::rt::{template, Part, Template};
pub use value_bag::ValueBag;

pub struct RawTimestamp(#[cfg(feature = "std")] pub Duration);

impl RawTimestamp {
    pub fn now() -> Self {
        #[cfg(feature = "std")]
        {
            use crate::std::time::SystemTime;

            RawTimestamp(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default(),
            )
        }
        #[cfg(not(feature = "std"))]
        {
            RawTimestamp()
        }
    }
}

#[derive(PartialEq, Eq)]
pub struct RawLevel(pub u8);

impl RawLevel {
    pub const DEBUG: Self = RawLevel(7);
    pub const INFO: Self = RawLevel(6);
    pub const WARN: Self = RawLevel(4);
    pub const ERROR: Self = RawLevel(3);
}

impl Default for RawLevel {
    fn default() -> Self {
        RawLevel::INFO
    }
}
