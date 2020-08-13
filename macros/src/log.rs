use std::{collections::BTreeMap, iter::Peekable, mem};

use crate::template::{Part, Template};
use proc_macro2::{Span, TokenStream, TokenTree};
use syn::{Expr, ExprPath, ExprStruct, FieldValue, Ident};

use crate::capture::FieldValueExt;

pub(super) fn rearrange_tokens(input: TokenStream) -> TokenStream {
    let mut input = input.into_iter().peekable();

    // Take any arguments up to the string template
    // These are control arguments for the log statement that aren't key-value pairs
    let mut parsing_value = false;
    let (args, template) = take_until(&mut input, |tt, _| {
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

    let template = template.expect("missing string template");

    // If there's more tokens, they should be a comma followed by comma-separated field-values
    if input.peek().is_some() {
        expect_punct(&mut input, ',');
        let kvs: TokenStream = input.collect();

        quote!(antlog_macros::__private_log!(#template Args { #args kvs: KeyValues { #kvs } }))
    } else {
        quote!(antlog_macros::__private_log!(#template))
    }
}

pub(super) fn expand_tokens(input: TokenStream) -> TokenStream {
    let mut input = input.into_iter().peekable();

    // Parse the template as the first argument
    // This doesn't run through `syn` so we can work with subspans on the literal directly
    let template_src = match input.next() {
        Some(TokenTree::Literal(template)) => template,
        _ => panic!("expected a string literal"),
    };

    let template = template_src.to_string();
    let template = Template::parse(&template).expect("failed to parse");

    let template_tokens = template.rt_tokens();

    // Extract the rest of the input arguments
    // These are parsed as a struct (the actual type doesn't matter)
    // Specific fields on this struct are interpreted as the log target and additional field-values
    let args: TokenStream = input.collect();

    // The log target expression
    let mut logger_tokens = None;
    // Any field-values that aren't part of the template
    let mut extra_field_values = BTreeMap::new();

    if !args.is_empty() {
        let args = syn::parse2::<ExprStruct>(args).expect("failed to parse");

        for field_value in args.fields {
            let key_name = field_value.key_name().expect("expected a string key");

            match key_name.as_str() {
                "logger" => logger_tokens = Some(field_value.expr),
                "kvs" => match field_value.expr {
                    Expr::Struct(kvs) => {
                        extra_field_values.extend(
                            kvs.fields
                                .into_iter()
                                .map(|kv| (kv.key_name().expect("expected a string key"), kv)),
                        );
                    }
                    _ => panic!("unexpected kvs value"),
                },
                _ => panic!("unexpected key in private macro"),
            }
        }
    }

    // TODO: Actually use the logger token
    let _ = logger_tokens;

    // The key-value expressions. These are extracted through a `match` expression
    let mut field_values = Vec::new();
    // The identifiers to bind key-values to. These are in the same order as `field_values`
    let mut field_bindings = Vec::new();
    // The identifiers key-values are bound to, sorted by the key so they can be binary searched
    let mut sorted_field_bindings = BTreeMap::new();
    let mut field_index = 0usize;

    let mut push_field_value = |key_name, mut expr: FieldValue, span: Span| {
        // TODO: Consider lifting attributes out to the top-level `match`:
        //
        // #[__log_private_apply(a, debug)]
        // #[__log_private_apply(b, ignore)]
        //
        // So that we can use attributes to entirely remove key-value pairs
        let attrs = mem::replace(&mut expr.attrs, vec![]);

        let value_expr = Ident::new(&format!("__tmp{}", field_index), span.clone());

        field_values
            .push(quote_spanned!(span=> #(#attrs)* antlog_macros::__private_log_capture!(#expr)));
        field_bindings.push(value_expr.clone());

        // Make sure keys aren't duplicated
        let previous = sorted_field_bindings.insert(key_name, value_expr);
        if previous.is_some() {
            panic!("keys cannot be duplicated");
        }

        field_index += 1;
    };

    // Push the field-values that appear in the template
    for part in template.parts.into_iter() {
        if let Part::Hole { expr, range } = part {
            let key_name = expr.key_name().expect("expected a string key");

            // If the hole has a corresponding field-value outside the template
            // then it will be used as the source for the value and attributes
            // In this case, it's expected that the field-value in the template is
            // just a single identifier
            let expr = match extra_field_values.remove(&key_name) {
                Some(extra_expr) => {
                    if let Expr::Path(ExprPath { ref path, .. }) = expr.expr {
                        // Make sure the field-value in the template is just a plain identifier
                        assert!(expr.attrs.is_empty(), "keys that exist in the template and extra pairs should only use attributes on the extra pair");
                        assert_eq!(
                            path.get_ident().map(|ident| ident.to_string()).as_ref(),
                            Some(&key_name),
                            "the key name and path don't match"
                        );
                    } else {
                        panic!("keys that exist in the template and extra pairs should only use identifiers");
                    }

                    extra_expr
                }
                None => expr,
            };

            push_field_value(
                key_name,
                expr,
                template_src
                    .subspan(range.start..range.end)
                    .unwrap_or_else(Span::call_site),
            );
        }
    }

    // Push any remaining extra field-values
    for (key_name, expr) in extra_field_values {
        push_field_value(key_name, expr, Span::call_site());
    }

    let sorted_field_bindings = sorted_field_bindings.values();

    quote!({
        match (#(#field_values),*) {
            (#(#field_bindings),*) => {
                let captured = antlog_macros_rt::__private::Captured {
                    sorted_key_values: &[#(#sorted_field_bindings),*]
                };

                let template = #template_tokens;

                println!("{:?}", captured.sorted_key_values);
                println!("{}", template.render(antlog_macros_rt::__private::Context::new().fill_source(&captured)));
            }
        }
    })
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

fn expect_punct(mut input: impl Iterator<Item = TokenTree>, c: char) -> TokenTree {
    match input.next() {
        Some(TokenTree::Punct(p)) if p.as_char() == c => TokenTree::Punct(p),
        _ => panic!("expected a {:?}", c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn rearrange_log() {
        let cases = vec![
            (
                quote!("There's no replacements here",
                    a,
                    b: 17,
                    #[debug]
                    c,
                    d: String::from("short lived!"),
                    #[error]
                    e
                ),
                quote!(antlog_macros::__private_log!("There's no replacements here" Args {
                    kvs: KeyValues {
                        a,
                        b: 17,
                        #[debug]
                        c,
                        d: String::from("short lived!"),
                        #[error]
                        e
                    }
                }))
            ),
            (
                quote!(log, "There's no replacements here", a),
                quote!(antlog_macros::__private_log!("There's no replacements here" Args {
                    log,
                    kvs: KeyValues { a }
                }))
            ),
            (
                quote!(log: { let x = "lol string"; x }, "There's no replacements here", a),
                quote!(antlog_macros::__private_log!("There's no replacements here" Args {
                    log: { let x = "lol string"; x },
                    kvs: KeyValues { a }
                }))
            )
        ];

        for (expr, expected) in cases {
            let actual = rearrange_tokens(expr);

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }

    #[test]
    #[rustfmt::skip]
    fn expand_log() {
        let cases = vec![
            (
                quote!("Text and {b: 17} and {a} and {#[debug] c} and {d: String::from(\"short lived\")}"),
                quote!({
                    match (
                        antlog_macros::__private_log_capture!(b: 17),
                        antlog_macros::__private_log_capture!(a),
                        #[debug]
                        antlog_macros::__private_log_capture!(c),
                        antlog_macros::__private_log_capture!(d: String::from("short lived"))
                    ) {
                        (__tmp0, __tmp1, __tmp2, __tmp3) => {
                            let captured = antlog_macros_rt::__private::Captured {
                                sorted_key_values: &[__tmp1, __tmp0, __tmp2, __tmp3]
                            };

                            let template = antlog_macros_rt::__private::build(&[
                                antlog_macros_rt::__private::Part::Text("Text and "),
                                antlog_macros_rt::__private::Part::Hole ( "b"),
                                antlog_macros_rt::__private::Part::Text(" and "),
                                antlog_macros_rt::__private::Part::Hole ( "a"),
                                antlog_macros_rt::__private::Part::Text(" and "),
                                antlog_macros_rt::__private::Part::Hole ( "c" ),
                                antlog_macros_rt::__private::Part::Text(" and "),
                                antlog_macros_rt::__private::Part::Hole ( "d" )
                            ]);

                            println!("{:?}", captured.sorted_key_values);
                            println!("{}", template.render(antlog_macros_rt::__private::Context::new().fill_source(&captured)));
                        }
                    }
                }),
            ),
            (
                quote!("Text and {a}" Args { logger, kvs: KeyValues { a: 42 } }),
                quote!({
                    match (
                        antlog_macros::__private_log_capture!(a: 42)
                    ) {
                        (__tmp0) => {
                            let captured = antlog_macros_rt::__private::Captured {
                                sorted_key_values: &[__tmp0]
                            };

                            let template = antlog_macros_rt::__private::build(&[
                                antlog_macros_rt::__private::Part::Text("Text and "),
                                antlog_macros_rt::__private::Part::Hole ( "a")
                            ]);

                            println!("{:?}", captured.sorted_key_values);
                            println!("{}", template.render(antlog_macros_rt::__private::Context::new().fill_source(&captured)));
                        }
                    }
                })
            )
        ];

        for (expr, expected) in cases {
            let actual = expand_tokens(expr);

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
