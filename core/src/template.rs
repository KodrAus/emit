use core::fmt;

use crate::{empty::Empty, props::Props, value::Value};

#[derive(Clone)]
pub struct Template<'a>(&'a [Part<'a>]);

impl<'a> fmt::Debug for Template<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.render(), f)
    }
}

impl<'a> fmt::Display for Template<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.render(), f)
    }
}

impl<'a> Template<'a> {
    pub fn new(parts: &'static [Part<'static>]) -> Template<'a> {
        Template(parts)
    }

    pub fn new_ref(parts: &'a [Part<'a>]) -> Template<'a> {
        Template(parts)
    }

    pub fn by_ref<'b>(&'b self) -> Template<'b> {
        Template(self.0)
    }

    pub fn render<'b>(&'b self) -> Render<'b, Empty> {
        Render {
            tpl: self.by_ref(),
            props: Empty,
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
}

impl<'a, P: Props> Render<'a, P> {
    pub fn write(&self, mut writer: impl Write) -> fmt::Result {
        for part in self.tpl.0 {
            match part.0 {
                PartKind::Text { value } => writer.write_text(value)?,
                PartKind::Hole { label, formatter } => {
                    if let Some(value) = self.props.get(label) {
                        if let Some(formatter) = formatter {
                            writer.write_hole_fmt(value, formatter)?;
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
        struct FormatValue<'a> {
            value: Value<'a>,
            formatter: Formatter,
        }

        impl<'a> fmt::Display for FormatValue<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                (self.formatter)(&self.value, f)
            }
        }

        self.write_fmt(format_args!("{}", FormatValue { value, formatter }))
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
        formatter(&value, self)
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
