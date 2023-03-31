use crate::std::fmt;

pub struct Record<'a> {
    pub timestamp: Timestamp,
    pub level: Level,
    pub kvs: &'a [(&'static str, ValueBag<'a>)],
    pub template: Template<'a>,
}

impl<'a> Record<'a> {
    pub fn get(&self, key: impl AsRef<str>) -> Option<&ValueBag<'a>> {
        self.kvs
            .binary_search_by_key(&key.as_ref(), |(k, _)| k)
            .ok()
            .map(|index| &self.kvs[index].1)
    }
}

impl<'a> fmt::Display for Record<'a> {
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

pub struct Timestamp;

impl Timestamp {
    pub fn now() -> Self {
        Timestamp
    }
}

#[derive(PartialEq, Eq)]
pub struct Level(pub u8);

impl Level {
    pub const DEBUG: Self = Level(7);
    pub const INFO: Self = Level(6);
    pub const WARN: Self = Level(4);
    pub const ERROR: Self = Level(3);
}

impl Default for Level {
    fn default() -> Self {
        Level::INFO
    }
}
