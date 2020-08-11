/*!
Compile-time string template parsing.
*/

use std::{borrow::Cow, fmt, iter::Peekable, ops::Range, str::CharIndices};

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
    Text { text: Cow<'a, str>, range: Range<usize> },
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

    fn invalid_literal() -> Self {
        Error {
            reason: format!("templates must be parsed from string literals"),
            source: None,
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

    The input string should be a raw, unescaped Rust string literal.
    So:

    ```text
    "A string with \"embedded escapes\""
    ```

    instead of:

    ```text
    A string with "embedded escapes"
    ```
    */
    pub fn parse(input: &'a str) -> Result<Self, Error> {
        enum Expecting {
            TextOrEOF,
            Hole,
        }

        struct Scan<'input> {
            input: &'input str,
            start: usize,
            end: usize,
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
            ) -> Result<Option<(Cow<'input, str>, Range<usize>)>, Error> {
                let mut scan = || {
                    while let Some((i, c)) = self.iter.next() {
                        if until_true(c, &mut self.iter)? {
                            let start = self.start;
                            let end = i;

                            self.start = end + 1;

                            let range = start..end;
                            return Ok((Cow::Borrowed(&self.input[range.clone()]), range));
                        }
                    }

                    let range = self.start..self.end;
                    Ok((Cow::Borrowed(&self.input[range.clone()]), range))
                };

                match scan()? {
                    (s, r) if s.len() > 0 => Ok(Some((s, r))),
                    _ => Ok(None),
                }
            }

            fn take_until_eof_or_hole_start(
                &mut self,
            ) -> Result<Option<(Cow<'input, str>, Range<usize>)>, Error> {
                let mut escaped = false;
                let scanned = self.take_until(|c, rest| match c {
                    // A `{` that's followed by another `{` is escaped
                    // If it's followed by a different character then it's
                    // the start of an interpolated expression
                    '{' => match rest.peek().map(|(_, peeked)| *peeked) {
                        Some('{') => {
                            escaped = true;
                            let _ = rest.next();
                            Ok(false)
                        }
                        Some(_) => Ok(true),
                        None => Err(Error::incomplete_hole()),
                    },
                    // A `}` that's followed by another `}` is escaped
                    // We should never see these in this parser unless they're escaped
                    // If we do it means an interpolated expression is missing its start
                    // or it's been improperly escaped
                    '}' => match rest.peek().map(|(_, peeked)| *peeked) {
                        Some('}') => {
                            escaped = true;
                            let _ = rest.next();
                            Ok(false)
                        }
                        Some(_) => Err(Error::unescaped_hole()),
                        None => Err(Error::unescaped_hole()),
                    },
                    _ => Ok(false),
                })?;

                match scanned {
                    Some((input, range)) if escaped => {
                        // If the input is escaped, then replace `{{` and `}}` chars
                        let input = (&*input).replace("{{", "{").replace("}}", "}");
                        Ok(Some((Cow::Owned(input), range)))
                    },
                    scanned => Ok(scanned),
                }
            }

            fn take_until_hole_end(
                &mut self,
            ) -> Result<Option<(Cow<'input, str>, Range<usize>)>, Error> {
                let mut depth = 1;
                let mut matched_hole_end = false;
                let mut escaped = false;

                let scanned = self.take_until(|c, _| {
                    // NOTE: This isn't perfect, it will fail for `{` and `}` within strings:
                    // "Hello {#[log::debug] "some { string"}"
                    match c {
                        // If the depth would return to its start then we've got a full expression
                        '}' if depth == 1 => {
                            matched_hole_end = true;
                            Ok(true)
                        }
                        // A block end will reduce the depth
                        '}' => {
                            depth -= 1;
                            Ok(false)
                        }
                        // A block start will increase the depth
                        '{' => {
                            depth += 1;
                            Ok(false)
                        }
                        // A `\` means there's embedded escaped characters
                        // For strings, we're only interested in `\"`
                        '\\' => {
                            escaped = true;
                            Ok(false)
                        },
                        _ => Ok(false),
                    }
                })?;

                if !matched_hole_end {
                    Err(Error::incomplete_hole())?;
                }

                match scanned {
                    Some((input, range)) if escaped => {
                        // If the input is escaped then replace `\"` with `"`
                        let input = (&*input).replace("\\\"", "\"");
                        Ok(Some((Cow::Owned(input), range)))
                    },
                    scanned => Ok(scanned),
                }
            }
        }

        let mut parts = Vec::new();
        let mut expecting = Expecting::TextOrEOF;

        if input.len() == 0 {
            return Ok(Template { raw: input, parts });
        }

        let mut iter = input.char_indices();
        let start = iter.next();
        let end = iter.next_back();

        // This just checks that we're looking at a string
        // It doesn't bother with ensuring that last quote is unescaped
        // because the input to this is expected to be a proc-macro literal
        if start.map(|(_, c)| c) != Some('"') || end.map(|(_, c)| c) != Some('"') {
            return Err(Error::invalid_literal());
        }

        let mut scan = Scan {
            input,
            start: 1,
            end: input.len() - 1,
            iter: iter.peekable(),
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
                                syn::parse_str(&*expr).map_err(|e| Error::parse_expr(&*expr, e))?;
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

    /**
    Generate an expression that will build a runtime template from this parsed one.

    The runtime template does not need to allocate, everything is borrowed in-place.
    */
    pub fn rt_tokens(&self) -> TokenStream {
        let parts = self.parts.iter().map(|part| match part {
            Part::Text { text, .. } => quote!(antlog_macros_rt::__private::Part::Text(#text)),
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

                quote!(antlog_macros_rt::__private::Part::Hole(#label))
            }
        });

        quote!(
            antlog_macros_rt::__private::build(&[#(#parts),*])
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
            ("\"Hello world 🎈📌\"", vec![text("Hello world 🎈📌", 1..21)]),
            (
                "\"Hello {world} 🎈📌\"",
                vec![
                    text("Hello ", 1..7),
                    hole("world", 8..13),
                    text(" 🎈📌", 14..23),
                ],
            ),
            ("\"{world}\"", vec![hole("world", 2..7)]),
            (
                "\"Hello {#[log::debug] world} 🎈📌\"",
                vec![
                    text("Hello ", 1..7),
                    hole("#[log::debug] world", 8..27),
                    text(" 🎈📌", 28..37),
                ],
            ),
            (
                "\"Hello {#[log::debug] world: 42} 🎈📌\"",
                vec![
                    text("Hello ", 1..7),
                    hole("#[log::debug] world: 42", 8..31),
                    text(" 🎈📌", 32..41),
                ],
            ),
            (
                "\"Hello {#[log::debug] world: \\\"is text\\\"} 🎈📌\"",
                vec![
                    text("Hello ", 1..7),
                    hole("#[log::debug] world: \"is text\"", 8..40),
                    text(" 🎈📌", 41..50),
                ],
            ),
            (
                "\"{Hello} {world}\"",
                vec![hole("Hello", 2..7), text(" ", 8..9), hole("world", 10..15)],
            ),
            (
                "\"{a}{b}{c}\"",
                vec![hole("a", 2..3), hole("b", 5..6), hole("c", 8..9)],
            ),
            (
                "\"🎈📌{a}🎈📌{b}🎈📌{c}🎈📌\"",
                vec![
                    text("🎈📌", 1..9),
                    hole("a", 10..11),
                    text("🎈📌", 12..20),
                    hole("b", 21..22),
                    text("🎈📌", 23..31),
                    hole("c", 32..33),
                    text("🎈📌", 34..42),
                ],
            ),
            (
                "\"Hello 🎈📌 {{world}}\"",
                vec![text("Hello 🎈📌 {world}", 1..25)],
            ),
            (
                "\"🎈📌 Hello world {{}}\"",
                vec![text("🎈📌 Hello world {}", 1..26)],
            ),
            ("\"{{\"", vec![text("{", 1..3)]),
            ("\"}}\"", vec![text("}", 1..3)]),
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
            ("\"", "parsing failed: templates must be parsed from string literals"),
            ("\"{\"", "parsing failed: unexpected end of input, expected `}`"),
            ("\"a {\"", "parsing failed: unexpected end of input, expected `}`"),
            ("\"a { a\"", "parsing failed: unexpected end of input, expected `}`"),
            ("\"{ a\"", "parsing failed: unexpected end of input, expected `}`"),
            ("\"}\"", "parsing failed: `{` and `}` characters must be escaped as `{{` and `}}`"),
            ("\"} a\"", "parsing failed: `{` and `}` characters must be escaped as `{{` and `}}`"),
            ("\"a } a\"", "parsing failed: `{` and `}` characters must be escaped as `{{` and `}}`"),
            ("\"a }\"", "parsing failed: `{` and `}` characters must be escaped as `{{` and `}}`"),
            ("\"{}\"", "parsing failed: empty replacements (`{}`) aren\'t supported, put the replacement inside like `{some_value}`"),
            ("\"{not real rust}\"", "parsing failed: failed to parse `not real rust` as an expression"),
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
    fn rt_tokens() {
        let cases = vec![(
            "\"Hello {#[log::debug] world}!\"",
            quote!(antlog_macros_rt::__private::build(&[
                antlog_macros_rt::__private::Part::Text("Hello "),
                antlog_macros_rt::__private::Part::Hole("world"),
                antlog_macros_rt::__private::Part::Text("!")
            ])),
        )];

        for (template, expected) in cases {
            let template = Template::parse(template).expect("failed to parse template");

            let actual = template.rt_tokens();
            assert_eq!(expected.to_string(), actual.to_string());
        }
    }

    fn text(text: &str, range: Range<usize>) -> Part {
        Part::Text { text: Cow::Borrowed(text), range }
    }

    fn hole(expr: &str, range: Range<usize>) -> Part {
        Part::Hole {
            expr: syn::parse_str(expr).expect("failed to parse expr"),
            range,
        }
    }
}
