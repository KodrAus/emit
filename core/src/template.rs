use core::{fmt, marker::PhantomData};

use crate::{empty::Empty, props::Props, value::Value};

#[derive(Clone)]
pub struct Template<'a>(TemplateKind<'a>);

#[derive(Clone)]
enum TemplateKind<'a> {
    Literal([Part<'a>; 1]),
    Parts(&'a [Part<'a>]),
}

impl<'a> TemplateKind<'a> {
    fn parts(&self) -> &[Part] {
        match self {
            TemplateKind::Literal(ref parts) => parts,
            TemplateKind::Parts(parts) => parts,
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

impl<'a> Template<'a> {
    pub fn new(parts: &'static [Part<'static>]) -> Template<'a> {
        Template(TemplateKind::Parts(parts))
    }

    pub fn new_ref(parts: &'a [Part<'a>]) -> Template<'a> {
        Template(TemplateKind::Parts(parts))
    }

    pub fn literal(text: &'static str) -> Template<'a> {
        Template(TemplateKind::Literal([Part::text(text)]))
    }

    pub fn literal_ref(text: &'a str) -> Template<'a> {
        Template(TemplateKind::Literal([Part::text_ref(text)]))
    }

    pub fn by_ref<'b>(&'b self) -> Template<'b> {
        match self.0 {
            TemplateKind::Literal([ref part]) => Template(TemplateKind::Literal([part.by_ref()])),
            TemplateKind::Parts(parts) => Template(TemplateKind::Parts(parts)),
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self.0.parts() {
            [Part(PartKind::Text { value, .. })] => Some(value),
            _ => None,
        }
    }

    pub fn as_static_str(&self) -> Option<&'static str> {
        match self.0.parts() {
            [Part(PartKind::Text {
                value_static: Some(value),
                ..
            })] => Some(value),
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
            match part.0 {
                PartKind::Text {
                    value,
                    value_static: _,
                } => writer.write_text(value)?,
                PartKind::Hole {
                    label,
                    ref formatter,
                    label_static: _,
                } => {
                    if let Some(value) = self.props.get(label) {
                        if let Some(formatter) = formatter {
                            writer.write_hole_fmt(value, formatter.by_ref())?;
                        } else {
                            writer.write_hole_value(value)?;
                        }
                    } else {
                        writer.write_hole_label(label)?;
                    }
                }
            }
        }

        Ok(())
    }
}

pub trait Write: fmt::Write {
    fn write_text(&mut self, text: &str) -> fmt::Result {
        self.write_str(text)
    }

    fn write_hole_value(&mut self, value: Value) -> fmt::Result {
        self.write_fmt(format_args!("{}", value))
    }

    fn write_hole_fmt(&mut self, value: Value, formatter: Formatter) -> fmt::Result {
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

    fn write_hole_value(&mut self, value: Value) -> fmt::Result {
        (**self).write_hole_value(value)
    }

    fn write_hole_fmt(&mut self, value: Value, formatter: Formatter) -> fmt::Result {
        (**self).write_hole_fmt(value, formatter)
    }

    fn write_hole_label(&mut self, label: &str) -> fmt::Result {
        (**self).write_hole_label(label)
    }
}

impl<'a> Write for fmt::Formatter<'a> {
    fn write_hole_value(&mut self, value: Value) -> fmt::Result {
        fmt::Display::fmt(&value, self)
    }

    fn write_hole_fmt(&mut self, value: Value, formatter: Formatter) -> fmt::Result {
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

        struct Escape<W>(W);

        impl<W: fmt::Write> fmt::Write for Escape<W> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                for c in s.escape_debug() {
                    self.0.write_char(c)?;
                }

                Ok(())
            }
        }

        f.write_char('"')?;
        write!(Escape(&mut *f), "{}", self)?;
        f.write_char('"')
    }
}

#[derive(Clone)]
pub struct Part<'a>(PartKind<'a>);

impl<'a> Part<'a> {
    pub fn text(text: &'static str) -> Part<'a> {
        Part(PartKind::Text {
            value: text,
            value_static: Some(text),
        })
    }

    pub fn text_ref(text: &'a str) -> Part<'a> {
        Part(PartKind::Text {
            value: text,
            value_static: None,
        })
    }

    pub fn hole(label: &'static str) -> Part<'a> {
        Part(PartKind::Hole {
            label,
            label_static: Some(label),
            formatter: None,
        })
    }

    pub fn hole_ref(label: &'a str) -> Part<'a> {
        Part(PartKind::Hole {
            label,
            label_static: None,
            formatter: None,
        })
    }

    pub fn by_ref<'b>(&'b self) -> Part<'b> {
        match self.0 {
            PartKind::Text {
                value,
                value_static,
            } => Part(PartKind::Text {
                value,
                value_static,
            }),
            PartKind::Hole {
                label,
                label_static,
                ref formatter,
            } => Part(PartKind::Hole {
                label,
                label_static,
                formatter: formatter.as_ref().map(|formatter| formatter.by_ref()),
            }),
        }
    }

    pub fn with_formatter(self, formatter: Formatter<'a>) -> Self {
        match self.0 {
            PartKind::Text {
                value,
                value_static,
            } => Part(PartKind::Text {
                value,
                value_static,
            }),
            PartKind::Hole {
                label,
                label_static,
                formatter: _,
            } => Part(PartKind::Hole {
                label,
                label_static,
                formatter: Some(formatter),
            }),
        }
    }
}

#[derive(Clone)]
pub struct Formatter<'a> {
    fmt: fn(Value, &mut fmt::Formatter) -> fmt::Result,
    _marker: PhantomData<&'a dyn Fn(Value, &mut fmt::Formatter) -> fmt::Result>,
}

impl<'a> Formatter<'a> {
    pub fn new(fmt: fn(Value, &mut fmt::Formatter) -> fmt::Result) -> Self {
        Formatter {
            fmt,
            _marker: PhantomData,
        }
    }

    pub fn by_ref<'b>(&'b self) -> Formatter<'b> {
        Formatter {
            fmt: self.fmt,
            _marker: PhantomData,
        }
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
        value: &'a str,
        value_static: Option<&'static str>,
    },
    Hole {
        label: &'a str,
        label_static: Option<&'static str>,
        formatter: Option<Formatter<'a>>,
    },
}
