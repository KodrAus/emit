use emit_core::{
    event::Event,
    filter::Filter,
    props::Props,
    runtime::InternalFilter,
    value::FromValue,
    well_known::{KEY_LVL, LVL_DEBUG, LVL_ERROR, LVL_INFO, LVL_WARN},
};

use crate::value::{ToValue, Value};
use core::{fmt, str::FromStr};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Debug for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Level::Info => LVL_INFO,
            Level::Error => LVL_ERROR,
            Level::Warn => LVL_WARN,
            Level::Debug => LVL_DEBUG,
        })
    }
}

impl FromStr for Level {
    type Err = ParseLevelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lvl = s.as_bytes();

        match lvl.get(0) {
            Some(b'I') | Some(b'i') => parse(lvl, b"INFORMATION", Level::Info),
            Some(b'D') | Some(b'd') => {
                parse(lvl, b"DEBUG", Level::Debug).or_else(|_| parse(lvl, b"DBG", Level::Debug))
            }
            Some(b'E') | Some(b'e') => parse(lvl, b"ERROR", Level::Error),
            Some(b'W') | Some(b'w') => {
                parse(lvl, b"WARNING", Level::Warn).or_else(|_| parse(lvl, b"WRN", Level::Warn))
            }
            Some(_) => Err(ParseLevelError {}),
            None => Err(ParseLevelError {}),
        }
    }
}

fn parse(
    mut input: &[u8],
    mut expected_uppercase: &[u8],
    ok: Level,
) -> Result<Level, ParseLevelError> {
    // Assume the first character has already been matched
    input = &input[1..];
    expected_uppercase = &expected_uppercase[1..];

    // Doesn't require a full match of the expected content
    // For example, `INF` will match `INFORMATION`
    while let Some(b) = input.get(0) {
        let Some(e) = expected_uppercase.get(0) else {
            return Err(ParseLevelError {});
        };

        if b.to_ascii_uppercase() != *e {
            return Err(ParseLevelError {});
        }

        expected_uppercase = &expected_uppercase[1..];
        input = &input[1..];
    }

    Ok(ok)
}

#[derive(Debug)]
pub struct ParseLevelError {}

impl Default for Level {
    fn default() -> Self {
        Level::Info
    }
}

impl ToValue for Level {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> FromValue<'v> for Level {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value
            .downcast_ref::<Level>()
            .copied()
            .or_else(|| value.parse())
    }
}

pub fn min_filter(min: Level) -> MinLevel {
    MinLevel::new(min)
}

pub struct MinLevel {
    min: Level,
    default: Level,
}

impl From<Level> for MinLevel {
    fn from(min: Level) -> Self {
        MinLevel::new(min)
    }
}

impl MinLevel {
    pub const fn new(min: Level) -> MinLevel {
        MinLevel {
            min,
            default: Level::Debug,
        }
    }

    pub fn treat_unleveled_as(mut self, default: Level) -> Self {
        self.default = default;
        self
    }
}

impl Filter for MinLevel {
    fn matches<P: Props>(&self, evt: &Event<P>) -> bool {
        evt.props()
            .pull::<Level, _>(KEY_LVL)
            .unwrap_or(self.default)
            >= self.min
    }
}

impl InternalFilter for MinLevel {}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    use alloc::vec::Vec;

    use emit_core::path::Path;

    pub fn min_by_path_filter<P: Into<Path<'static>>, L: Into<MinLevel>>(
        levels: impl IntoIterator<Item = (P, L)>,
    ) -> MinLevelPathMap {
        MinLevelPathMap::from_iter(levels)
    }

    pub struct MinLevelPathMap {
        paths: Vec<(Path<'static>, MinLevel)>,
    }

    impl MinLevelPathMap {
        pub const fn new() -> Self {
            MinLevelPathMap { paths: Vec::new() }
        }

        pub fn min_level(
            &mut self,
            path: impl Into<Path<'static>>,
            min_level: impl Into<MinLevel>,
        ) {
            let path = path.into();

            match self.paths.binary_search_by_key(&&path, |(path, _)| path) {
                Ok(index) => {
                    self.paths[index] = (path, min_level.into());
                }
                Err(index) => {
                    self.paths.insert(index, (path, min_level.into()));
                }
            }
        }
    }

    impl Filter for MinLevelPathMap {
        fn matches<P: Props>(&self, evt: &Event<P>) -> bool {
            let evt_path = evt.module();

            if let Ok(index) = self.paths.binary_search_by_key(&evt_path, |(path, _)| path) {
                self.paths[index].1.matches(evt)
            } else {
                for (path, min_level) in &self.paths {
                    if evt_path.is_child_of(path) {
                        return min_level.matches(evt);
                    }
                }

                true
            }
        }
    }

    impl InternalFilter for MinLevelPathMap {}

    impl<P: Into<Path<'static>>, L: Into<MinLevel>> FromIterator<(P, L)> for MinLevelPathMap {
        fn from_iter<T: IntoIterator<Item = (P, L)>>(iter: T) -> Self {
            let mut map = MinLevelPathMap::new();

            for (path, min_level) in iter {
                map.min_level(path, min_level);
            }

            map
        }
    }
}

#[cfg(feature = "alloc")]
pub use self::alloc_support::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_roundtrip() {
        for lvl in [Level::Info, Level::Debug, Level::Warn, Level::Error] {
            let fmt = lvl.to_string();

            let parsed: Level = fmt.parse().unwrap();

            assert_eq!(lvl, parsed, "{}", fmt);
        }
    }
}
