use core::fmt;

use crate::Value;

#[derive(Clone)]
pub struct Template<'a>(&'a [Part<'a>]);

impl<'a> Template<'a> {
    pub fn new(parts: &'static [Part<'static>]) -> Template<'a> {
        Template(parts)
    }

    pub fn by_ref<'b>(&'b self) -> Template<'b> {
        Template(self.0)
    }
}

#[derive(Clone)]
pub struct Part<'a>(PartKind<'a>);

impl<'a> Part<'a> {
    pub fn text(text: &'static str) -> Part<'a> {
        Part(PartKind::Text { value: text })
    }

    pub fn hole(label: &'static str) -> Part<'a> {
        Part(PartKind::Hole { label, fmt: None })
    }
}

#[derive(Clone)]
enum PartKind<'a> {
    Text {
        value: &'a str,
    },
    Hole {
        label: &'a str,
        fmt: Option<fn(&Value, &mut fmt::Formatter) -> fmt::Result>,
    },
}
