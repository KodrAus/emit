/*!
Runtime string template formatting.
*/

use std::fmt;

/**
A text template.
*/
pub struct Template<'a> {
    parts: &'a [Part<'a>],
}

impl<'a> Template<'a> {
    /**
    Render the template using the given context.

    The context helps the template find
    */
    pub fn render<'brw, 'ctx_fill, 'ctx_missing>(
        &'brw self,
        ctx: Context<'ctx_fill, 'ctx_missing>,
    ) -> impl fmt::Display + 'brw
    where
        'ctx_fill: 'brw,
        'ctx_missing: 'brw,
    {
        struct ImplDisplay<'tpl, 'brw, 'ctx_fill, 'ctx_missing> {
            template: &'brw Template<'tpl>,
            ctx: Context<'ctx_fill, 'ctx_missing>,
        }

        impl<'tpl, 'brw, 'ctx_fill, 'ctx_missing> fmt::Display
            for ImplDisplay<'tpl, 'brw, 'ctx_fill, 'ctx_missing>
        {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                for part in self.template.parts {
                    match part {
                        Part::Text(text) => f.write_str(text)?,
                        Part::Hole(label) => {
                            if let Some(r) = (self.ctx.fill)(f, label) {
                                r?;
                            } else {
                                (self.ctx.missing)(f, label)?;
                            }
                        }
                    }
                }

                Ok(())
            }
        }

        ImplDisplay {
            template: self,
            ctx,
        }
    }
}

/**
A context used to render a template.
*/
pub struct Context<'fill, 'missing> {
    fill: &'fill dyn Fn(&mut dyn fmt::Write, &str) -> Option<fmt::Result>,
    missing: &'missing dyn Fn(&mut dyn fmt::Write, &str) -> fmt::Result,
}

impl<'fill, 'missing> Context<'fill, 'missing> {
    /**
    Create a new, empty context.
    */
    pub fn new() -> Context<'fill, 'missing> {
        Context {
            fill: &|_, _| None,
            missing: &|f, label| f.write_fmt(format_args!("`{}`", label)),
        }
    }

    /**
    Provide a function to fill the holes in the template with.
    */
    pub fn fill<'a>(
        self,
        fill: &'a impl Fn(&mut dyn fmt::Write, &str) -> Option<fmt::Result>,
    ) -> Context<'a, 'missing> {
        Context {
            fill,
            missing: self.missing,
        }
    }

    /**
    Provide a function to handle unfilled holes.
    */
    pub fn missing<'a>(
        self,
        missing: &'a impl Fn(&mut dyn fmt::Write, &str) -> fmt::Result,
    ) -> Context<'fill, 'a> {
        Context {
            fill: self.fill,
            missing,
        }
    }
}

impl<'fill, 'missing> Default for Context<'fill, 'missing> {
    fn default() -> Self {
        Self::new()
    }
}

pub enum Part<'a> {
    Text(&'a str),
    Hole(&'a str),
}

pub fn build<'a>(parts: &'a [Part<'a>]) -> Template<'a> {
    Template { parts }
}

#[cfg(test)]
mod tests {
    use super::*;

    use log::kv::{Key, Source, Value};

    #[test]
    fn render() {
        let cases = vec![
            (
                &[Part::Text("Hello "), Part::Hole("world"), Part::Text("!")],
                Context::new().fill(&|write, label| Some(write.write_str(label))),
                "Hello world!",
            ),
            (
                &[Part::Text("Hello "), Part::Hole("world"), Part::Text("!")],
                Context::new(),
                "Hello `world`!",
            ),
            (
                &[Part::Text("Hello "), Part::Hole("world"), Part::Text("!")],
                Context::new()
                    .missing(&|write, label| write.write_fmt(format_args!("{{{}}}", label))),
                "Hello {world}!",
            ),
        ];

        for (parts, ctx, expected) in cases {
            let template = build(parts);

            let actual = template.render(ctx).to_string();

            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn render_source() {
        let cases = vec![
            (
                &[Part::Text("Hello "), Part::Hole("world"), Part::Text("!")],
                vec![("world", Value::from(42))],
                "Hello 42!",
            ),
            (
                &[Part::Text("Hello "), Part::Hole("world"), Part::Text("!")],
                vec![],
                "Hello `world`!",
            ),
        ];

        for (parts, source, expected) in cases {
            let template = build(parts);

            let actual = template
                .render(Context::new().fill(&|write, label| {
                    Source::get(&source, Key::from(label))
                        .map(|value| write.write_fmt(format_args!("{}", value)))
                }))
                .to_string();

            assert_eq!(expected, actual);
        }
    }
}
