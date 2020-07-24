/*!
A different string template parser.

The shape of templates accepted by this crate are different from `std::fmt`.
It doesn't have any direct understanding of formatting flags.
Instead, it just parses field expressions between braces in a string and
leaves it up to a consumer to decide what to do with them.
*/

use std::{fmt, iter::Peekable, str::CharIndices};

use quote::ToTokens;
use syn::FieldValue;
use thiserror::Error;

/**
A parsed template.
*/
#[derive(Debug)]
pub struct Template<'a> {
    pub raw: &'a str,
    pub parts: Vec<Part<'a>>,
}

/**
A part of a parsed template.
*/
pub enum Part<'a> {
    /**
    A fragment of text.
    */
    Text(&'a str),
    /**
    A replacement expression.
    */
    Hole(FieldValue),
}

/**
An error encountered while parsing a template.
*/
#[derive(Error, Debug)]
#[error("parsing failed: {reason}")]
pub struct Error {
    reason: String,
    source: Option<Box<dyn std::error::Error>>,
    // TODO: Source span (position or range)
}

impl Error {
    fn incomplete_hole() -> Self {
        Error {
            reason: format!("unexpected end of input, expected `}}`"),
            source: None,
        }
    }

    fn unescaped_hole() -> Self {
        Error {
            reason: format!("`{{` and `}}` characters must be escaped as `{{{{` and `}}}}`"),
            source: None,
        }
    }

    fn missing_expr() -> Self {
        Error {
            reason: format!("empty replacements (`{{}}`) aren't supported, put the replacement inside like `{{some_value}}`"),
            source: None,
        }
    }

    fn parse_expr(expr: &str, err: syn::Error) -> Self {
        Error {
            reason: format!("failed to parse `{}` as an expression", expr),
            source: Some(err.into()),
        }
    }
}

impl<'a> fmt::Debug for Part<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Part::Text(text) => text.fmt(f),
            Part::Hole(expr) => f.write_fmt(format_args!("`{}`", expr.to_token_stream())),
        }
    }
}

impl<'a> Template<'a> {
    /**
    Try to parse a template into its parts.
    */
    pub fn parse(input: &'a str) -> Result<Self, Error> {
        enum Expecting {
            TextOrEOF,
            Hole,
        }

        struct Scan<'input> {
            input: &'input str,
            start: usize,
            iter: Peekable<CharIndices<'input>>,
        }

        impl<'input> Scan<'input> {
            fn has_input(&mut self) -> bool {
                self.iter.peek().is_some()
            }

            fn take_until(
                &mut self,
                mut until_true: impl FnMut(
                    char,
                    &mut Peekable<CharIndices<'input>>,
                ) -> Result<bool, Error>,
            ) -> Result<Option<&'input str>, Error> {
                let mut scan = || {
                    while let Some((i, c)) = self.iter.next() {
                        if until_true(c, &mut self.iter)? {
                            let start = self.start;
                            let end = i;

                            self.start = end + 1;
                            return Ok(&self.input[start..end]);
                        }
                    }

                    Ok(&self.input[self.start..])
                };

                match scan()? {
                    s if s.len() > 0 => Ok(Some(s)),
                    _ => Ok(None),
                }
            }

            fn take_until_eof_or_hole_start(&mut self) -> Result<Option<&'input str>, Error> {
                self.take_until(|c, rest| match c {
                    '{' => match rest.peek().map(|(_, peeked)| *peeked) {
                        Some('{') => {
                            let _ = rest.next();
                            Ok(false)
                        }
                        Some(_) => Ok(true),
                        None => Err(Error::incomplete_hole()),
                    },
                    '}' => match rest.peek().map(|(_, peeked)| *peeked) {
                        Some('}') => {
                            let _ = rest.next();
                            Ok(false)
                        }
                        Some(_) => Err(Error::unescaped_hole()),
                        None => Err(Error::unescaped_hole()),
                    },
                    _ => Ok(false),
                })
            }

            fn take_until_hole_end(&mut self) -> Result<Option<&'input str>, Error> {
                let mut depth = 1;
                let mut matched_hole_end = false;

                let expr = self.take_until(|c, _| {
                    // NOTE: This isn't perfect, it will fail for `{` and `}` within strings:
                    // "Hello {#[log::debug] "some { string"}"
                    match c {
                        '}' if depth == 1 => {
                            matched_hole_end = true;
                            Ok(true)
                        }
                        '}' => {
                            depth -= 1;
                            Ok(false)
                        }
                        '{' => {
                            depth += 1;
                            Ok(false)
                        }
                        _ => Ok(false),
                    }
                })?;

                if !matched_hole_end {
                    Err(Error::incomplete_hole())?;
                }

                Ok(expr)
            }
        }

        let mut parts = Vec::new();
        let mut expecting = Expecting::TextOrEOF;

        let mut scan = Scan {
            input,
            start: 0,
            iter: input.char_indices().peekable(),
        };

        while scan.has_input() {
            match expecting {
                Expecting::TextOrEOF => {
                    if let Some(text) = scan.take_until_eof_or_hole_start()? {
                        parts.push(Part::Text(text));
                    }

                    expecting = Expecting::Hole;
                    continue;
                }
                Expecting::Hole => {
                    match scan.take_until_hole_end()? {
                        Some(expr) => {
                            let expr =
                                syn::parse_str(expr).map_err(|e| Error::parse_expr(expr, e))?;
                            parts.push(Part::Hole(expr));
                        }
                        None => Err(Error::missing_expr())?,
                    }

                    expecting = Expecting::TextOrEOF;
                    continue;
                }
            }
        }

        Ok(Template { raw: input, parts })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ok() {
        let cases = vec![
            ("", vec![]),
            ("Hello world 🎈📌", vec![text("Hello world 🎈📌")]),
            (
                "Hello {world} 🎈📌",
                vec![text("Hello "), hole("world"), text(" 🎈📌")],
            ),
            ("{world}", vec![hole("world")]),
            (
                "Hello {#[log::debug] world} 🎈📌",
                vec![text("Hello "), hole("#[log::debug] world"), text(" 🎈📌")],
            ),
            (
                "Hello {#[log::debug] world: 42} 🎈📌",
                vec![
                    text("Hello "),
                    hole("#[log::debug] world: 42"),
                    text(" 🎈📌"),
                ],
            ),
            (
                "Hello {#[log::debug] world: \"is text\"} 🎈📌",
                vec![
                    text("Hello "),
                    hole("#[log::debug] world: \"is text\""),
                    text(" 🎈📌"),
                ],
            ),
            (
                "{Hello} {world}",
                vec![hole("Hello"), text(" "), hole("world")],
            ),
            ("{a}{b}{c}", vec![hole("a"), hole("b"), hole("c")]),
            (
                "🎈📌{a}🎈📌{b}🎈📌{c}🎈📌",
                vec![
                    text("🎈📌"),
                    hole("a"),
                    text("🎈📌"),
                    hole("b"),
                    text("🎈📌"),
                    hole("c"),
                    text("🎈📌"),
                ],
            ),
            ("Hello 🎈📌 {{world}}", vec![text("Hello 🎈📌 {{world}}")]),
            ("🎈📌 Hello world {{}}", vec![text("🎈📌 Hello world {{}}")]),
            ("{{", vec![text("{{")]),
            ("}}", vec![text("}}")]),
        ];

        for (template, expected) in cases {
            let actual = match Template::parse(template) {
                Ok(template) => template,
                Err(e) => panic!("failed to parse {:?}: {}", template, e),
            };

            assert_eq!(
                format!(
                    "{:?}",
                    Template {
                        raw: template,
                        parts: expected
                    }
                ),
                format!("{:?}", actual),
                "parsing template: {:?}",
                template
            );
        }
    }

    #[test]
    fn parse_err() {
        let cases = vec![
            ("{", "parsing failed: unexpected end of input, expected `}`"),
            ("a {", "parsing failed: unexpected end of input, expected `}`"),
            ("a { a", "parsing failed: unexpected end of input, expected `}`"),
            ("{ a", "parsing failed: unexpected end of input, expected `}`"),
            ("}", "parsing failed: `{` and `}` characters must be escaped as `{{` and `}}`"),
            ("} a", "parsing failed: `{` and `}` characters must be escaped as `{{` and `}}`"),
            ("a } a", "parsing failed: `{` and `}` characters must be escaped as `{{` and `}}`"),
            ("a }", "parsing failed: `{` and `}` characters must be escaped as `{{` and `}}`"),
            ("{}", "parsing failed: empty replacements (`{}`) aren\'t supported, put the replacement inside like `{some_value}`"),
            ("{not real rust}", "parsing failed: failed to parse `not real rust` as an expression"),
        ];

        for (template, expected) in cases {
            let actual = match Template::parse(template) {
                Err(e) => e,
                Ok(template) => panic!("parsing should've failed but produced {:?}", template),
            };

            assert_eq!(
                expected,
                actual.to_string(),
                "parsing template: {:?}",
                template
            );
        }
    }

    fn text(text: &str) -> Part {
        Part::Text(text)
    }

    fn hole(expr: &str) -> Part {
        Part::Hole(syn::parse_str(expr).expect("failed to parse expr"))
    }
}
