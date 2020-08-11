use std::{collections::BTreeMap, mem};

use crate::template::{Part, Template};
use proc_macro2::{Span, TokenStream, TokenTree};
use syn::Ident;

use crate::capture::FieldValueExt;

pub(super) fn expand_tokens(expr: TokenStream) -> TokenStream {
    let mut expr = expr.into_iter();

    // Parse the template as the first argument
    // This doesn't run through `syn` so we can work with subspans on the literal directly
    let template_src = match expr.next() {
        Some(TokenTree::Literal(template)) => template,
        _ => panic!("expected a string literal"),
    };

    let template = template_src.to_string();
    let template = Template::parse(&template).expect("failed to parse");

    let template_tokens = template.rt_tokens();

    // TODO: parse the rest as a comma-separated list of field values
    // `logger`, `kvs`

    // The key-value expressions. These are extracted through a `match` expression
    let mut field_values = Vec::new();

    // The identifiers to bind key-values to. These are in the same order as `field_values`
    let mut field_bindings = Vec::new();

    // The identifiers key-values are bound to, sorted by the key so they can be binary searched
    let mut sorted_field_bindings = BTreeMap::new();

    let mut field_index = 0usize;
    for part in template.parts.into_iter() {
        if let Part::Hole { mut expr, range } = part {
            // TODO: Consider lifting attributes out to the top-level `match`:
            //
            // #[__log_private_apply(a, debug)]
            // #[__log_private_apply(b, ignore)]
            //
            // So that we can use attributes to entirely remove key-value pairs
            let attrs = mem::replace(&mut expr.attrs, vec![]);

            let field_span = template_src
                .subspan(range.start..range.end)
                .unwrap_or_else(Span::call_site);

            let key_name = expr.key_name().expect("expected a string key");

            let value_expr = Ident::new(&format!("__tmp{}", field_index), Span::call_site());

            field_values.push(quote_spanned!(field_span=> #(#attrs)* antlog_macros::__private_log_capture!(#expr)));
            field_bindings.push(value_expr.clone());
            sorted_field_bindings.insert(key_name, value_expr);

            field_index += 1;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

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
            )
        ];

        for (expr, expected) in cases {
            let actual = expand_tokens(expr);

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
