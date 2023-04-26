use core::fmt;

use crate::Value;

#[derive(Clone)]
pub struct Template<'a>(&'a [Part<'a>]);

impl<'a> Template<'a> {
    pub fn new(parts: &'a [Part<'static>]) -> Template<'a> {
        Template(parts)
    }

    pub fn new_ref(parts: &'a [Part<'a>]) -> Template<'a> {
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

    pub fn text_ref(text: &'a str) -> Part<'a> {
        Part(PartKind::Text { value: text })
    }

    pub fn hole(label: &'static str) -> Part<'a> {
        Part(PartKind::Hole {
            label,
            formatter: None,
        })
    }

    pub fn hole_ref(label: &'a str) -> Part<'a> {
        Part(PartKind::Hole {
            label,
            formatter: None,
        })
    }

    pub fn with_formatter(self, formatter: Formatter) -> Self {
        match self.0 {
            PartKind::Text { value } => Part(PartKind::Text { value }),
            PartKind::Hole {
                label,
                formatter: _,
            } => Part(PartKind::Hole {
                label,
                formatter: Some(formatter),
            }),
        }
    }
}

pub type Formatter = fn(&Value, &mut fmt::Formatter) -> fmt::Result;

#[derive(Clone)]
enum PartKind<'a> {
    Text {
        value: &'a str,
    },
    Hole {
        label: &'a str,
        formatter: Option<Formatter>,
    },
}
