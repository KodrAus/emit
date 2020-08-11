use std::{collections::BTreeMap, mem, ops::Range};

use crate::template::{Part, Template};
use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use syn::{Expr, ExprPath, ExprStruct, FieldValue, Ident};

use crate::capture::FieldValueExt;

pub(super) fn rearrange_tokens(input: TokenStream) -> TokenStream {
    let mut input = input.into_iter().peekable();

    // The first argument is the string literal template
    let template = expect_string_literal(&mut input);

    // If there's more tokens, they should be a comma followed by key-value pairs
    if input.peek().is_some() {
        expect_punct(&mut input, ',');
        let kvs = expect_group(&mut input, Delimiter::Brace);

        quote!(antlog_macros::__private_log!(#template Args { kvs: KeyValues #kvs }))
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

    // Exctract the rest of the input arguments
    let args: TokenStream = input.collect();

    let mut logger_tokens = None;
    let mut extra_field_values = BTreeMap::new();

    // If there are extra arguments following the template then parse them
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

    let mut push_field_value =
        |key_name, mut expr: FieldValue, template_range: Option<Range<usize>>| {
            // TODO: Consider lifting attributes out to the top-level `match`:
            //
            // #[__log_private_apply(a, debug)]
            // #[__log_private_apply(b, ignore)]
            //
            // So that we can use attributes to entirely remove key-value pairs
            let attrs = mem::replace(&mut expr.attrs, vec![]);

            let field_span = template_range
                .and_then(|template_range| {
                    template_src.subspan(template_range.start..template_range.end)
                })
                .unwrap_or_else(Span::call_site);

            let value_expr = Ident::new(&format!("__tmp{}", field_index), Span::call_site());

            field_values.push(quote_spanned!(field_span=> #(#attrs)* antlog_macros::__private_log_capture!(#expr)));
            field_bindings.push(value_expr.clone());

            let previous = sorted_field_bindings.insert(key_name, value_expr);
            if previous.is_some() {
                panic!("keys cannot be duplicated");
            }

            field_index += 1;
        };

    // Push the field values that appear in the template
    for part in template.parts.into_iter() {
        if let Part::Hole { expr, range } = part {
            let key_name = expr.key_name().expect("expected a string key");

            // If the hole
            let expr = match extra_field_values.remove(&key_name) {
                Some(extra_expr) => {
                    if let Expr::Path(ExprPath { ref path, .. }) = expr.expr {
                        let ident = path.get_ident().expect("");

                        assert!(expr.attrs.is_empty(), "keys that exist in the template and extra pairs should only use attributes on the extra pair");
                        assert_eq!(
                            ident.to_string(),
                            key_name,
                            "the key name and path don't match"
                        );
                    } else {
                        panic!("keys that exist in the template and extra pairs should only use identifiers");
                    }

                    extra_expr
                }
                None => expr,
            };

            push_field_value(key_name, expr, Some(range));
        }
    }

    // Push any remaining extra field values
    for (key_name, expr) in extra_field_values {
        push_field_value(key_name, expr, None);
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

fn expect_string_literal(mut input: impl Iterator<Item = TokenTree>) -> TokenTree {
    match input.next() {
        Some(TokenTree::Literal(l)) => TokenTree::Literal(l),
        _ => panic!("expected a string literal"),
    }
}

fn expect_punct(mut input: impl Iterator<Item = TokenTree>, c: char) -> TokenTree {
    match input.next() {
        Some(TokenTree::Punct(p)) if p.as_char() == c => TokenTree::Punct(p),
        _ => panic!("expected a {:?}", c),
    }
}

fn expect_group(mut input: impl Iterator<Item = TokenTree>, delim: Delimiter) -> TokenTree {
    match input.next() {
        Some(TokenTree::Group(g)) if g.delimiter() == delim => TokenTree::Group(g),
        _ => panic!("expected a {:?} separated group", delim),
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
                quote!("There's no replacements here", {
                    a,
                    b: 17,
                    #[debug]
                    c,
                    d: String::from("short lived!"),
                    #[error]
                    e
                }),
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
