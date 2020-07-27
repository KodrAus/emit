/*!
Compile-time string template parsing.
*/

use std::{fmt, iter::Peekable, ops::Range, str::CharIndices};

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{ExprLit, FieldValue, Lit, LitStr, Member};
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
    Text { text: &'a str, range: Range<usize> },
    /**
    A replacement expression.
    */
    Hole {
        expr: FieldValue,
        range: Range<usize>,
    },
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
            Part::Text { text, range } => f
                .debug_struct("Text")
                .field("text", text)
                .field("range", range)
                .finish(),
            Part::Hole { expr, range } => f
                .debug_struct("Hole")
                .field("expr", &format_args!("`{}`", expr.to_token_stream()))
                .field("range", range)
                .finish(),
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
            ) -> Result<Option<(&'input str, Range<usize>)>, Error> {
                let mut scan = || {
                    while let Some((i, c)) = self.iter.next() {
                        if until_true(c, &mut self.iter)? {
                            let start = self.start;
                            let end = i;

                            self.start = end + 1;

                            let range = start..end;
                            return Ok((&self.input[range.clone()], range));
                        }
                    }

                    let range = self.start..self.input.len();
                    Ok((&self.input[range.clone()], range))
                };

                match scan()? {
                    (s, r) if s.len() > 0 => Ok(Some((s, r))),
                    _ => Ok(None),
                }
            }

            fn take_until_eof_or_hole_start(
                &mut self,
            ) -> Result<Option<(&'input str, Range<usize>)>, Error> {
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

            fn take_until_hole_end(
                &mut self,
            ) -> Result<Option<(&'input str, Range<usize>)>, Error> {
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
                    if let Some((text, range)) = scan.take_until_eof_or_hole_start()? {
                        parts.push(Part::Text { text, range });
                    }

                    expecting = Expecting::Hole;
                    continue;
                }
                Expecting::Hole => {
                    match scan.take_until_hole_end()? {
                        Some((expr, range)) => {
                            let expr =
                                syn::parse_str(expr).map_err(|e| Error::parse_expr(expr, e))?;
                            parts.push(Part::Hole { expr, range });
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

    pub fn generate_rt(&self) -> TokenStream {
        let parts = self.parts.iter().map(|part| match part {
            Part::Text { text, .. } => quote!(antlog_template::__private::Part::Text(#text)),
            Part::Hole { expr, .. } => {
                let label = ExprLit {
                    attrs: vec![],
                    lit: Lit::Str(match expr.member {
                        Member::Named(ref member) => {
                            LitStr::new(&member.to_string(), member.span())
                        }
                        Member::Unnamed(ref member) => {
                            LitStr::new(&member.index.to_string(), member.span)
                        }
                    }),
                };

                quote!(antlog_template::__private::Part::Hole(#label))
            }
        });

        quote!(
            antlog_template::__private::build(&[#(#parts),*])
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ok() {
        let cases = vec![
            ("", vec![]),
            ("Hello world 🎈📌", vec![text("Hello world 🎈📌", 0..20)]),
            (
                "Hello {world} 🎈📌",
                vec![
                    text("Hello ", 0..6),
                    hole("world", 7..12),
                    text(" 🎈📌", 13..22),
                ],
            ),
            ("{world}", vec![hole("world", 1..6)]),
            (
                "Hello {#[log::debug] world} 🎈📌",
                vec![
                    text("Hello ", 0..6),
                    hole("#[log::debug] world", 7..26),
                    text(" 🎈📌", 27..36),
                ],
            ),
            (
                "Hello {#[log::debug] world: 42} 🎈📌",
                vec![
                    text("Hello ", 0..6),
                    hole("#[log::debug] world: 42", 7..30),
                    text(" 🎈📌", 31..40),
                ],
            ),
            (
                "Hello {#[log::debug] world: \"is text\"} 🎈📌",
                vec![
                    text("Hello ", 0..6),
                    hole("#[log::debug] world: \"is text\"", 7..37),
                    text(" 🎈📌", 38..47),
                ],
            ),
            (
                "{Hello} {world}",
                vec![hole("Hello", 1..6), text(" ", 7..8), hole("world", 9..14)],
            ),
            (
                "{a}{b}{c}",
                vec![hole("a", 1..2), hole("b", 4..5), hole("c", 7..8)],
            ),
            (
                "🎈📌{a}🎈📌{b}🎈📌{c}🎈📌",
                vec![
                    text("🎈📌", 0..8),
                    hole("a", 9..10),
                    text("🎈📌", 11..19),
                    hole("b", 20..21),
                    text("🎈📌", 22..30),
                    hole("c", 31..32),
                    text("🎈📌", 33..41),
                ],
            ),
            (
                "Hello 🎈📌 {{world}}",
                vec![text("Hello 🎈📌 {{world}}", 0..24)],
            ),
            (
                "🎈📌 Hello world {{}}",
                vec![text("🎈📌 Hello world {{}}", 0..25)],
            ),
            ("{{", vec![text("{{", 0..2)]),
            ("}}", vec![text("}}", 0..2)]),
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

    #[test]
    fn into_rt() {
        let cases = vec![(
            "Hello {#[log::debug] world}!",
            quote!(antlog_template::__private::build(&[
                antlog_template::__private::Part::Text("Hello "),
                antlog_template::__private::Part::Hole("world"),
                antlog_template::__private::Part::Text("!")
            ])),
        )];

        for (template, expected) in cases {
            let template = Template::parse(template).expect("failed to parse template");

            let actual = template.generate_rt();
            assert_eq!(expected.to_string(), actual.to_string());
        }
    }

    fn text(text: &str, range: Range<usize>) -> Part {
        Part::Text { text, range }
    }

    fn hole(expr: &str, range: Range<usize>) -> Part {
        Part::Hole {
            expr: syn::parse_str(expr).expect("failed to parse expr"),
            range,
        }
    }
}
