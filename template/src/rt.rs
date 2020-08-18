/*!
Runtime string template formatting.
*/

use std::fmt;

/**
A runtime field-value template.
*/
pub struct Template<'a> {
    parts: &'a [Part<'a>],
}

impl<'a> Template<'a> {
    /**
    Render the template using the given context.

    The context helps the template find replacement values and determines how to render them if they're missing.
    An empty context can be used to render out the template with just its holes.
    */
    pub fn render<'brw>(
        &'brw self,
        ctx: Context<
            impl (Fn(&mut fmt::Formatter, &str) -> Option<fmt::Result>) + 'brw,
            impl (Fn(&mut fmt::Formatter, &str) -> fmt::Result) + 'brw,
        >,
    ) -> impl fmt::Display + 'brw {
        struct ImplDisplay<'tpl, 'brw, TFill, TMissing> {
            template: &'brw Template<'tpl>,
            ctx: Context<TFill, TMissing>,
        }

        impl<'tpl, 'brw, TFill, TMissing> fmt::Display for ImplDisplay<'tpl, 'brw, TFill, TMissing>
        where
            TFill: Fn(&mut fmt::Formatter, &str) -> Option<fmt::Result>,
            TMissing: Fn(&mut fmt::Formatter, &str) -> fmt::Result,
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
pub struct Context<TFill, TMissing> {
    fill: TFill,
    missing: TMissing,
}

impl
    Context<
        fn(&mut fmt::Formatter, &str) -> Option<fmt::Result>,
        fn(&mut fmt::Formatter, &str) -> fmt::Result,
    >
{
    pub fn new() -> Self {
        Context {
            fill: |_, _| None,
            missing: |f, label| f.write_fmt(format_args!("`{}`", label)),
        }
    }
}

impl<TFill, TMissing> Context<TFill, TMissing>
where
    TFill: Fn(&mut fmt::Formatter, &str) -> Option<fmt::Result>,
    TMissing: Fn(&mut fmt::Formatter, &str) -> fmt::Result,
{
    /**
    Provide a function to fill the holes in the template with.
    */
    pub fn fill<T>(self, fill: T) -> Context<T, TMissing>
    where
        T: Fn(&mut fmt::Formatter, &str) -> Option<fmt::Result>,
    {
        Context {
            fill,
            missing: self.missing,
        }
    }

    /**
    Provide a function to handle unfilled holes.
    */
    pub fn missing<T>(self, missing: T) -> Context<TFill, T>
    where
        T: Fn(&mut fmt::Formatter, &str) -> fmt::Result,
    {
        Context {
            fill: self.fill,
            missing,
        }
    }
}

impl Default
    for Context<
        fn(&mut fmt::Formatter, &str) -> Option<fmt::Result>,
        fn(&mut fmt::Formatter, &str) -> fmt::Result,
    >
{
    fn default() -> Self {
        Self::new()
    }
}

pub enum Part<'a> {
    Text(&'a str),
    Hole(&'a str),
}

pub fn template<'a>(parts: &'a [Part<'a>]) -> Template<'a> {
    Template { parts }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render() {
        let cases = vec![
            (
                &[Part::Text("Hello "), Part::Hole("world"), Part::Text("!")],
                Context::new().fill(
                    (|write, label| Some(write.write_str(label)))
                        as fn(&mut fmt::Formatter, &str) -> Option<fmt::Result>,
                ),
                "Hello world!",
            ),
            (
                &[Part::Text("Hello "), Part::Hole("world"), Part::Text("!")],
                Context::new(),
                "Hello `world`!",
            ),
            (
                &[Part::Text("Hello "), Part::Hole("world"), Part::Text("!")],
                Context::new().missing(
                    (|write, label| write.write_fmt(format_args!("{{{}}}", label)))
                        as fn(&mut fmt::Formatter, &str) -> fmt::Result,
                ),
                "Hello {world}!",
            ),
        ];

        for (parts, ctx, expected) in cases {
            let template = template(parts);

            let actual = template.render(ctx).to_string();

            assert_eq!(expected, actual);
        }
    }
}
