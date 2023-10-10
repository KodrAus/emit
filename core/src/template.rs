use core::{fmt, marker::PhantomData};

use crate::{
    empty::Empty,
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

impl Part<'static> {
    pub const fn text(text: &'static str) -> Self {
        Part(PartKind::Text {
            value: text as *const str,
            value_static: Some(text),
            #[cfg(feature = "alloc")]
            value_owned: None,
            _marker: PhantomData,
        })
    }

    pub const fn hole(label: &'static str) -> Self {
        Part(PartKind::Hole {
            label: label as *const str,
            label_static: Some(label),
            formatter: None,
            #[cfg(feature = "alloc")]
            label_owned: None,
            _marker: PhantomData,
        })
    }
}

impl<'a> Part<'a> {
    pub const fn text_ref(text: &'a str) -> Self {
        Part(PartKind::Text {
            value: text as *const str,
            value_static: None,
            #[cfg(feature = "alloc")]
            value_owned: None,
            _marker: PhantomData,
        })
    }

    pub const fn hole_ref(label: &'a str) -> Self {
        Part(PartKind::Hole {
            label: label as *const str,
            label_static: None,
            #[cfg(feature = "alloc")]
            label_owned: None,
            formatter: None,
            _marker: PhantomData,
        })
    }

    pub const fn as_str(&self) -> Option<&str> {
        match self.0 {
            PartKind::Text { value, .. } => Some(unsafe { &*value }),
            _ => None,
        }
    }

    pub fn by_ref<'b>(&'b self) -> Part<'b> {
        match self.0 {
            PartKind::Text {
                value,
                value_static,
                ..
            } => Part(PartKind::Text {
                value,
                value_static,
                #[cfg(feature = "alloc")]
                value_owned: None,
                _marker: PhantomData,
            }),
            PartKind::Hole {
                label,
                label_static,
                ref formatter,
                ..
            } => Part(PartKind::Hole {
                label,
                label_static,
                #[cfg(feature = "alloc")]
                label_owned: None,
                formatter: formatter.clone(),
                _marker: PhantomData,
            }),
        }
    }

    pub fn with_formatter(self, formatter: Formatter) -> Self {
        match self.0 {
            #[cfg(not(feature = "alloc"))]
            PartKind::Hole {
                label,
                label_static,
                formatter: _,
                _marker,
            } => Part(PartKind::Hole {
                label,
                label_static,
                formatter: Some(formatter),
                _marker,
            }),
            #[cfg(feature = "alloc")]
            PartKind::Hole {
                label,
                label_static,
                label_owned,
                formatter: _,
                _marker,
            } => Part(PartKind::Hole {
                label,
                label_static,
                label_owned,
                formatter: Some(formatter),
                _marker: PhantomData,
            }),
            part => Part(part),
        }
    }

    fn write(&self, mut writer: impl Write, props: impl Props) -> fmt::Result {
        match self.0 {
            PartKind::Text { value, .. } => writer.write_text(unsafe { &*value }),
            PartKind::Hole {
                label,
                ref formatter,
                ..
            } => {
                let label = unsafe { &*label };

                if let Some(value) = props.get(label) {
                    if let Some(formatter) = formatter {
                        writer.write_hole_fmt(value, formatter.clone())
                    } else {
                        writer.write_hole_value(value)
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

enum PartKind<'a> {
    Text {
        value: *const str,
        value_static: Option<&'static str>,
        #[cfg(feature = "alloc")]
        value_owned: Option<Box<str>>,
        _marker: PhantomData<&'a str>,
    },
    Hole {
        label: *const str,
        label_static: Option<&'static str>,
        #[cfg(feature = "alloc")]
        label_owned: Option<Box<str>>,
        formatter: Option<Formatter>,
        _marker: PhantomData<&'a str>,
    },
}

unsafe impl<'a> Send for PartKind<'a> {}
unsafe impl<'a> Sync for PartKind<'a> {}

impl<'a> Clone for PartKind<'a> {
    fn clone(&self) -> Self {
        match self {
            #[cfg(feature = "alloc")]
            PartKind::Text {
                value,
                value_static,
                value_owned,
                _marker,
            } => match value_owned {
                Some(value_owned) => {
                    let value_owned = value_owned.clone();

                    PartKind::Text {
                        value: &*value_owned as *const str,
                        value_static: None,
                        value_owned: Some(value_owned),
                        _marker: PhantomData,
                    }
                }
                None => PartKind::Text {
                    value: *value,
                    value_static: *value_static,
                    value_owned: None,
                    _marker: PhantomData,
                },
            },
            #[cfg(not(feature = "alloc"))]
            PartKind::Text {
                value,
                value_static,
                _marker,
            } => PartKind::Text {
                value: *value,
                value_static: *value_static,
                _marker: PhantomData,
            },
            #[cfg(feature = "alloc")]
            PartKind::Hole {
                label,
                label_static,
                label_owned,
                ref formatter,
                _marker,
            } => match label_owned {
                Some(label_owned) => {
                    let label_owned = label_owned.clone();

                    PartKind::Hole {
                        label: &*label_owned as *const str,
                        label_static: None,
                        label_owned: Some(label_owned),
                        formatter: formatter.clone(),
                        _marker: PhantomData,
                    }
                }
                None => PartKind::Hole {
                    label: *label,
                    label_static: *label_static,
                    label_owned: None,
                    formatter: formatter.clone(),
                    _marker: PhantomData,
                },
            },
            #[cfg(not(feature = "alloc"))]
            PartKind::Hole {
                label,
                label_static,
                ref formatter,
                _marker,
            } => PartKind::Hole {
                label: *label,
                label_static: *label_static,
                formatter: formatter.clone(),
                _marker: PhantomData,
            },
        }
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

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
            let value_owned = text.into();

            Part(PartKind::Text {
                value: &*value_owned as *const str,
                value_static: None,
                value_owned: Some(value_owned),
                _marker: PhantomData,
            })
        }

        pub fn hole_owned(label: impl Into<Box<str>>) -> Self {
            let label_owned = label.into();

            Part(PartKind::Hole {
                label: &*label_owned as *const str,
                label_static: None,
                label_owned: Some(label_owned),
                formatter: None,
                _marker: PhantomData,
            })
        }
    }

    impl<'a> Part<'a> {
        fn to_owned(&self) -> Part<'static> {
            match self.0 {
                PartKind::Text {
                    value_static: Some(value),
                    ..
                } => Part::text(value),
                PartKind::Text { value, .. } => {
                    let value_owned = unsafe { (*value).to_owned().into_boxed_str() };

                    Part(PartKind::Text {
                        value: &*value_owned as *const str,
                        value_static: None,
                        value_owned: Some(value_owned),
                        _marker: PhantomData,
                    })
                }
                PartKind::Hole {
                    label_static: Some(label),
                    ref formatter,
                    ..
                } => Part(PartKind::Hole {
                    label,
                    label_static: Some(label),
                    label_owned: None,
                    formatter: formatter.clone(),
                    _marker: PhantomData,
                }),
                PartKind::Hole {
                    label,
                    ref formatter,
                    ..
                } => {
                    let label_owned = unsafe { (*label).to_owned().into_boxed_str() };

                    Part(PartKind::Hole {
                        label: &*label_owned as *const str,
                        label_static: None,
                        label_owned: Some(label_owned),
                        formatter: formatter.clone(),
                        _marker: PhantomData,
                    })
                }
            }
        }
    }
}

#[cfg(feature = "alloc")]
pub use self::alloc_support::*;
