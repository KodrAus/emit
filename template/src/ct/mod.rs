use std::iter::Peekable;

use proc_macro2::{Literal, TokenStream, TokenTree};
use syn::{ExprLit, FieldValue, Lit, LitStr, Member};
use thiserror::Error;

mod parts;

use self::parts::Part;

pub struct Template {
    before_template: Vec<FieldValue>,
    template: Vec<Part>,
    after_template: Vec<FieldValue>,
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

impl Template {
    // TODO: Implement `syn::Parse` for this
    pub fn parse2(input: TokenStream) -> Result<Self, Error> {
        let mut input = input.into_iter().peekable();

        // Take any arguments up to the string template
        // These are control arguments for the log statement that aren't key-value pairs
        let mut parsing_value = false;
        let (before_template, template) = take_until(&mut input, |tt, _| {
            // If we're parsing a value then skip over this token
            // It won't be interpreted as the template because it belongs to an arg
            if parsing_value {
                parsing_value = false;
                return false;
            }

            match tt {
                // A literal is interpreted as the template
                TokenTree::Literal(_) => true,
                // A `:` token marks the start of a value in a field-value
                // The following token is the value, which isn't considered the template
                TokenTree::Punct(p) if p.as_char() == ':' => {
                    parsing_value = true;
                    false
                }
                // Any other token isn't the template
                _ => false,
            }
        });

        // If there's more tokens, they should be a comma followed by comma-separated field-values
        let after_template = if input.peek().is_some() {
            expect_punct(&mut input, ',');
            input.collect()
        } else {
            TokenStream::new()
        };

        let before_template = collect_field_values(before_template);
        let after_template = collect_field_values(after_template);

        let template = parts::parse(take_literal(template.expect("missing string template")))
            .expect("failed to parse");

        Ok(Template {
            before_template,
            template,
            after_template,
        })
    }

    pub fn before_template_field_values<'a>(&'a self) -> impl Iterator<Item = &'a FieldValue> {
        self.before_template.iter()
    }

    pub fn template_field_values<'a>(&'a self) -> impl Iterator<Item = &'a FieldValue> {
        self.template.iter().filter_map(|part| {
            if let Part::Hole { expr, .. } = part {
                Some(expr)
            } else {
                None
            }
        })
    }

    pub fn after_template_field_values<'a>(&'a self) -> impl Iterator<Item = &'a FieldValue> {
        self.after_template.iter()
    }

    pub fn to_rt_tokens(&self) -> TokenStream {
        let parts = self.template.iter().map(|part| match part {
            Part::Text { text, .. } => quote!(fv_template::rt::Part::Text(#text)),
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

                quote!(fv_template::rt::Part::Hole(#label))
            }
        });

        quote!(
            fv_template::rt::template(&[#(#parts),*])
        )
    }
}

fn take_until<I>(
    iter: &mut Peekable<I>,
    mut until_true: impl FnMut(&TokenTree, &mut Peekable<I>) -> bool,
) -> (TokenStream, Option<TokenTree>)
where
    I: Iterator<Item = TokenTree>,
{
    let mut taken = TokenStream::new();

    while let Some(tt) = iter.next() {
        if until_true(&tt, iter) {
            return (taken, Some(tt));
        }

        taken.extend(Some(tt));
    }

    (taken, None)
}

fn is_punct(input: &TokenTree, c: char) -> bool {
    match input {
        TokenTree::Punct(p) if p.as_char() == c => true,
        _ => false,
    }
}

fn expect_punct(mut input: impl Iterator<Item = TokenTree>, c: char) -> TokenTree {
    input
        .next()
        .filter(|input| is_punct(input, c))
        .unwrap_or_else(|| panic!("expected a {:?} character", c))
}

fn take_literal(input: TokenTree) -> Literal {
    match input {
        TokenTree::Literal(l) => l,
        _ => panic!("expected a literal"),
    }
}

fn collect_field_values(input: impl IntoIterator<Item = TokenTree>) -> Vec<FieldValue> {
    let mut iter = input.into_iter().peekable();
    let mut result = Vec::new();

    while iter.peek().is_some() {
        let (arg, _) = take_until(&mut iter, |tt, _| is_punct(&tt, ','));

        if !arg.is_empty() {
            result.push(syn::parse2::<FieldValue>(arg).unwrap());
        }
    }

    result
}
