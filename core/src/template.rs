use core::fmt;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::{
    empty::Empty,
    key::Key,
    props::Props,
    value::{ToValue, Value},
};

#[derive(Clone)]
pub struct Template<'a>(TemplateKind<'a>);

#[derive(Clone)]
enum TemplateKind<'a> {
    Literal([Part<'a>; 1]),
    Parts(&'a [Part<'a>]),
    #[cfg(feature = "alloc")]
    Owned(Box<[Part<'static>]>),
}

impl<'a> TemplateKind<'a> {
    fn parts(&self) -> &[Part] {
        match self {
            TemplateKind::Literal(ref parts) => parts,
            TemplateKind::Parts(parts) => parts,
            #[cfg(feature = "alloc")]
            TemplateKind::Owned(parts) => parts,
        }
    }
}

impl<'a> fmt::Debug for Template<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.render(Empty), f)
    }
}

impl<'a> fmt::Display for Template<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.render(Empty), f)
    }
}

impl Template<'static> {
    pub fn new(parts: &'static [Part<'static>]) -> Self {
        Template(TemplateKind::Parts(parts))
    }

    pub fn literal(text: &'static str) -> Self {
        Template(TemplateKind::Literal([Part::text(text)]))
    }
}

impl<'a> Template<'a> {
    pub fn new_ref(parts: &'a [Part<'a>]) -> Self {
        Template(TemplateKind::Parts(parts))
    }

    pub fn literal_ref(text: &'a str) -> Self {
        Template(TemplateKind::Literal([Part::text_ref(text)]))
    }

    pub fn by_ref<'b>(&'b self) -> Template<'b> {
        match self.0 {
            TemplateKind::Literal([ref part]) => Template(TemplateKind::Literal([part.by_ref()])),
            TemplateKind::Parts(parts) => Template(TemplateKind::Parts(parts)),
            #[cfg(feature = "alloc")]
            TemplateKind::Owned(ref parts) => Template(TemplateKind::Parts(parts)),
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self.0.parts() {
            [part] => part.as_str(),
            _ => None,
        }
    }

    pub fn as_static_str(&self) -> Option<&'static str> {
        match self.0.parts() {
            [Part(PartKind::Text { value, .. })] => value.as_static_str(),
            _ => None,
        }
    }

    pub fn render<'b, P>(&'b self, props: P) -> Render<'b, P> {
        Render {
            tpl: self.by_ref(),
            props,
        }
    }
}

impl<'a> ToValue for Template<'a> {
    fn to_value(&self) -> Value {
        if let Some(tpl) = self.as_str() {
            Value::from(tpl)
        } else {
            Value::from_display(self)
        }
    }
}

pub struct Render<'a, P> {
    tpl: Template<'a>,
    props: P,
}

impl<'a, P> Render<'a, P> {
    pub fn with_props<U>(self, props: U) -> Render<'a, U> {
        Render {
            tpl: self.tpl,
            props,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        self.tpl.as_str()
    }

    pub fn as_static_str(&self) -> Option<&'static str> {
        self.tpl.as_static_str()
    }
}

impl<'a, P: Props> Render<'a, P> {
    pub fn write(&self, mut writer: impl Write) -> fmt::Result {
        for part in self.tpl.0.parts() {
            part.write(&mut writer, &self.props)?;
        }

        Ok(())
    }
}

impl<'a, P: Props> ToValue for Render<'a, P> {
    fn to_value(&self) -> Value {
        if let Some(tpl) = self.as_str() {
            Value::from(tpl)
        } else {
            Value::from_display(self)
        }
    }
}

pub trait Write: fmt::Write {
    fn write_text(&mut self, text: &str) -> fmt::Result {
        self.write_str(text)
    }

    fn write_hole_value(&mut self, label: &str, value: Value) -> fmt::Result {
        let _ = label;
        self.write_fmt(format_args!("{}", value))
    }

    fn write_hole_fmt(&mut self, label: &str, value: Value, formatter: Formatter) -> fmt::Result {
        let _ = label;
        self.write_fmt(format_args!("{}", formatter.apply(value)))
    }

    fn write_hole_label(&mut self, label: &str) -> fmt::Result {
        self.write_fmt(format_args!("`{}`", label))
    }
}

impl<'a, W: Write + ?Sized> Write for &'a mut W {
    fn write_text(&mut self, text: &str) -> fmt::Result {
        (**self).write_text(text)
    }

    fn write_hole_value(&mut self, label: &str, value: Value) -> fmt::Result {
        (**self).write_hole_value(label, value)
    }

    fn write_hole_fmt(&mut self, label: &str, value: Value, formatter: Formatter) -> fmt::Result {
        (**self).write_hole_fmt(label, value, formatter)
    }

    fn write_hole_label(&mut self, label: &str) -> fmt::Result {
        (**self).write_hole_label(label)
    }
}

impl<'a> Write for fmt::Formatter<'a> {
    fn write_hole_value(&mut self, _: &str, value: Value) -> fmt::Result {
        fmt::Display::fmt(&value, self)
    }

    fn write_hole_fmt(&mut self, _: &str, value: Value, formatter: Formatter) -> fmt::Result {
        formatter.fmt(value, self)
    }
}

impl<'a, P: Props> fmt::Display for Render<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write(f)
    }
}

impl<'a, P: Props> fmt::Debug for Render<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write as _;

        f.write_char('"')?;
        write!(WriteEscaped(&mut *f), "{}", self)?;
        f.write_char('"')
    }
}

struct WriteEscaped<W>(W);

impl<W: fmt::Write> WriteEscaped<W> {
    pub fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
        self.0.write_fmt(args)
    }
}

impl<W: fmt::Write> fmt::Write for WriteEscaped<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.escape_debug() {
            self.0.write_char(c)?;
        }

        Ok(())
    }
}

struct WriteBraced<W>(W);

impl<W: fmt::Write> WriteBraced<W> {
    pub fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
        self.0.write_fmt(args)
    }
}

impl<W: fmt::Write> fmt::Write for WriteBraced<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(s)
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.0.write_char(c)
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.write_fmt(args)
    }
}

impl<W: Write> Write for WriteBraced<W> {
    fn write_text(&mut self, text: &str) -> fmt::Result {
        self.0.write_text(text)
    }

    fn write_hole_value(&mut self, label: &str, value: Value) -> fmt::Result {
        self.0.write_hole_value(label, value)
    }

    fn write_hole_fmt(&mut self, label: &str, value: Value, formatter: Formatter) -> fmt::Result {
        self.0.write_hole_fmt(label, value, formatter)
    }

    fn write_hole_label(&mut self, label: &str) -> fmt::Result {
        self.0.write_fmt(format_args!("{{{}}}", label))
    }
}

pub struct Braced<'a, P>(Render<'a, P>);

impl<'a> Template<'a> {
    pub fn braced<'b>(&'b self) -> Braced<'b, Empty> {
        self.render(Empty).braced()
    }
}

impl<'a, P> Render<'a, P> {
    pub fn braced(self) -> Braced<'a, P> {
        Braced(self)
    }
}

impl<'a, P: Props> fmt::Display for Braced<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.write(WriteBraced(f))
    }
}

#[derive(Clone)]
pub struct Part<'a>(PartKind<'a>);

impl Part<'static> {
    pub const fn text(text: &'static str) -> Self {
        Part(PartKind::Text {
            value: Key::new(text),
        })
    }

    pub const fn hole(label: &'static str) -> Self {
        Part(PartKind::Hole {
            label: Key::new(label),
            formatter: None,
        })
    }
}

impl<'a> Part<'a> {
    pub const fn text_ref(text: &'a str) -> Self {
        Part(PartKind::Text {
            value: Key::new_ref(text),
        })
    }

    pub const fn hole_ref(label: &'a str) -> Self {
        Part(PartKind::Hole {
            label: Key::new_ref(label),
            formatter: None,
        })
    }

    pub const fn as_str(&self) -> Option<&str> {
        match self.0 {
            PartKind::Text { ref value, .. } => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn by_ref<'b>(&'b self) -> Part<'b> {
        match self.0 {
            PartKind::Text { ref value } => Part(PartKind::Text {
                value: value.by_ref(),
            }),
            PartKind::Hole {
                ref label,
                ref formatter,
            } => Part(PartKind::Hole {
                label: label.by_ref(),
                formatter: formatter.clone(),
            }),
        }
    }

    pub fn with_formatter(self, formatter: Formatter) -> Self {
        match self.0 {
            PartKind::Hole {
                label,
                formatter: _,
            } => Part(PartKind::Hole {
                label,
                formatter: Some(formatter),
            }),
            part => Part(part),
        }
    }

    fn write(&self, mut writer: impl Write, props: impl Props) -> fmt::Result {
        match self.0 {
            PartKind::Text { ref value, .. } => writer.write_text(value.as_str()),
            PartKind::Hole {
                ref label,
                ref formatter,
                ..
            } => {
                let label = label.as_str();

                if let Some(value) = props.get(label) {
                    if let Some(formatter) = formatter {
                        writer.write_hole_fmt(label, value, formatter.clone())
                    } else {
                        writer.write_hole_value(label, value)
                    }
                } else {
                    writer.write_hole_label(label)
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct Formatter {
    fmt: fn(Value, &mut fmt::Formatter) -> fmt::Result,
}

impl Formatter {
    pub fn new(fmt: fn(Value, &mut fmt::Formatter) -> fmt::Result) -> Self {
        Formatter { fmt }
    }

    pub fn fmt(&self, value: Value, f: &mut fmt::Formatter) -> fmt::Result {
        (self.fmt)(value, f)
    }

    pub fn apply<'b>(&'b self, value: Value<'b>) -> impl fmt::Display + 'b {
        struct FormatValue<'a> {
            value: Value<'a>,
            fmt: fn(Value, &mut fmt::Formatter) -> fmt::Result,
        }

        impl<'a> fmt::Display for FormatValue<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                (self.fmt)(self.value.by_ref(), f)
            }
        }

        FormatValue {
            value,
            fmt: self.fmt,
        }
    }
}

#[derive(Clone)]
enum PartKind<'a> {
    Text {
        value: Key<'a>,
    },
    Hole {
        label: Key<'a>,
        formatter: Option<Formatter>,
    },
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    use alloc::vec::Vec;

    impl Template<'static> {
        pub fn new_owned(parts: impl Into<Box<[Part<'static>]>>) -> Self {
            let parts = parts.into();

            Template(TemplateKind::Owned(parts))
        }
    }

    impl<'a> Template<'a> {
        pub fn to_owned(&self) -> Template<'static> {
            match self.0 {
                TemplateKind::Owned(ref parts) => Template::new_owned(parts.clone()),
                ref parts => {
                    let mut dst = Vec::new();

                    for part in parts.parts() {
                        dst.push(part.to_owned());
                    }

                    Template::new_owned(dst)
                }
            }
        }
    }

    impl Part<'static> {
        pub fn text_owned(text: impl Into<Box<str>>) -> Self {
            Part(PartKind::Text {
                value: Key::new_owned(text),
            })
        }

        pub fn hole_owned(label: impl Into<Box<str>>) -> Self {
            Part(PartKind::Hole {
                label: Key::new_owned(label),
                formatter: None,
            })
        }
    }

    impl<'a> Part<'a> {
        fn to_owned(&self) -> Part<'static> {
            match self.0 {
                PartKind::Text { ref value, .. } => Part(PartKind::Text {
                    value: value.to_owned(),
                }),
                PartKind::Hole {
                    ref label,
                    ref formatter,
                    ..
                } => Part(PartKind::Hole {
                    label: label.to_owned(),
                    formatter: formatter.clone(),
                }),
            }
        }
    }
}
