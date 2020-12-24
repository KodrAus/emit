use std::{collections::BTreeMap, mem};

use proc_macro2::TokenStream;
use syn::{spanned::Spanned, Expr, ExprPath, FieldValue, Ident};

use fv_template::ct::Template;

use crate::capture::FieldValueExt;

pub(super) fn expand_tokens(input: TokenStream) -> TokenStream {
    let template = Template::parse2(input).expect("failed to expand template");

    // Any field-values that aren't part of the template
    let mut extra_field_values: BTreeMap<_, _> = template
        .after_template_field_values()
        .map(|fv| (fv.key_name().expect("expected a string key"), fv))
        .collect();

    // A runtime representation of the template
    let template_tokens = template.to_rt_tokens(quote!(emit::__private));

    // The key-value expressions. These are extracted through a `match` expression
    let mut field_values = Vec::new();
    // The identifiers to bind key-values to. These are in the same order as `field_values`
    let mut field_bindings = Vec::new();
    // The identifiers key-values are bound to, sorted by the key so they can be binary searched
    let mut sorted_field_bindings = BTreeMap::new();
    let mut field_index = 0usize;

    let mut push_field_value = |k, mut fv: FieldValue| {
        // TODO: Consider lifting attributes out to the top-level `match`:
        //
        // #[__log_private_apply(a, debug)]
        // #[__log_private_apply(b, ignore)]
        //
        // So that we can use attributes to entirely remove key-value pairs
        let attrs = mem::replace(&mut fv.attrs, vec![]);

        let v = Ident::new(&format!("__tmp{}", field_index), fv.span());

        field_values
            .push(quote_spanned!(fv.span()=> #(#attrs)* emit::__private_capture!(#fv)));
        field_bindings.push(v.clone());

        // Make sure keys aren't duplicated
        let previous = sorted_field_bindings.insert(k, v);
        if previous.is_some() {
            panic!("keys cannot be duplicated");
        }

        field_index += 1;
    };

    // Push the field-values that appear in the template
    for fv in template.template_field_values() {
        let k = fv.key_name().expect("expected a string key");

        // If the hole has a corresponding field-value outside the template
        // then it will be used as the source for the value and attributes
        // In this case, it's expected that the field-value in the template is
        // just a single identifier
        let fv = match extra_field_values.remove(&k) {
            Some(extra_fv) => {
                if let Expr::Path(ExprPath { ref path, .. }) = fv.expr {
                    // Make sure the field-value in the template is just a plain identifier
                    assert!(fv.attrs.is_empty(), "keys that exist in the template and extra pairs should only use attributes on the extra pair");
                    assert_eq!(
                        path.get_ident().map(|ident| ident.to_string()).as_ref(),
                        Some(&k),
                        "the key name and path don't match"
                    );
                } else {
                    panic!("keys that exist in the template and extra pairs should only use identifiers");
                }

                extra_fv
            }
            None => fv,
        };

        push_field_value(k, fv.clone());
    }

    // Push any remaining extra field-values
    for (k, fv) in extra_field_values {
        push_field_value(k, fv.clone());
    }

    // The log target expression
    let log_tokens = template
        .before_template_field_values()
        .find(|fv| fv.key_name().map(|k| k.as_str() == "log").unwrap_or(false))
        .map(|fv| {
            let logger = &fv.expr;
            quote!(Some(#logger))
        })
        .unwrap_or_else(|| quote!(None));

    let field_value_tokens = field_values.iter();
    let field_binding_tokens = field_bindings.iter();
    let sorted_field_key_tokens = sorted_field_bindings.keys();
    let sorted_field_accessor_tokens = 0..sorted_field_bindings.len();
    let sorted_field_binding_tokens = sorted_field_bindings.values();

    quote!({
        match (#(#field_value_tokens),*) {
            (#(#field_binding_tokens),*) => {
                let source = emit::__private::Source {
                    sorted_key_values: &[#(#sorted_field_binding_tokens),*]
                };

                let template = #template_tokens;

                let record = emit::__private::Record {
                    source,
                    template,
                };

                emit::__private_forward!(
                    #log_tokens,
                    [#(#sorted_field_key_tokens),*],
                    [#(&record.source[#sorted_field_accessor_tokens]),*],
                    &record
                );
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn expand_emit() {
        let cases = vec![
            (
                quote!("Text and {b: 17} and {a} and {#[debug] c} and {d: String::from(\"short lived\")}"),
                quote!({
                    match (
                        emit::__private_capture!(b: 17),
                        emit::__private_capture!(a),
                        #[debug]
                        emit::__private_capture!(c),
                        emit::__private_capture!(d: String::from("short lived"))
                    ) {
                        (__tmp0, __tmp1, __tmp2, __tmp3) => {
                            let source = emit::__private::Source {
                                sorted_key_values: &[__tmp1, __tmp0, __tmp2, __tmp3]
                            };

                            let template = emit::__private::template(&[
                                emit::__private::Part::Text("Text and "),
                                emit::__private::Part::Hole ( "b"),
                                emit::__private::Part::Text(" and "),
                                emit::__private::Part::Hole ( "a"),
                                emit::__private::Part::Text(" and "),
                                emit::__private::Part::Hole ( "c" ),
                                emit::__private::Part::Text(" and "),
                                emit::__private::Part::Hole ( "d" )
                            ]);

                            let record = emit::__private::Record {
                                source,
                                template,
                            };

                            emit::__private_forward!(
                                None,
                                ["a", "b", "c", "d"],
                                [&record.source[0usize], &record.source[1usize], &record.source[2usize], &record.source[3usize]],
                                &record
                            );
                        }
                    }
                }),
            ),
            (
                quote!(log, "Text and {a}", a: 42),
                quote!({
                    match (
                        emit::__private_capture!(a: 42)
                    ) {
                        (__tmp0) => {
                            let source = emit::__private::Source {
                                sorted_key_values: &[__tmp0]
                            };

                            let template = emit::__private::template(&[
                                emit::__private::Part::Text("Text and "),
                                emit::__private::Part::Hole ( "a")
                            ]);

                            let record = emit::__private::Record {
                                source,
                                template,
                            };

                            emit::__private_forward!(
                                Some(log),
                                ["a"],
                                [&record.source[0usize]],
                                &record
                            );
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
