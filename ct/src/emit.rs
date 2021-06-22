/*!
Compile-time implementation of event emission.

This module generates calls to `rt::emit`.
*/

use std::{collections::BTreeMap, mem};

use proc_macro2::TokenStream;
use syn::{spanned::Spanned, Attribute, Expr, ExprPath, FieldValue, Ident};

use fv_template::ct::Template;

use crate::capture::FieldValueExt;

pub(super) fn expand_tokens(input: TokenStream) -> TokenStream {
    let record_ident = Ident::new(&"record", input.span());
    let template = Template::parse2(input).expect("failed to expand template");

    // Any field-values that aren't part of the template
    let mut extra_field_values: BTreeMap<_, _> = template
        .after_template_field_values()
        .map(|fv| (fv.key_name().expect("expected a string key"), fv))
        .collect();

    // The key-value expressions. These are extracted through a `match` expression
    let mut field_values = Vec::new();
    // The identifiers to bind key-values to. These are in the same order as `field_values`
    let mut field_bindings = Vec::new();
    // The identifiers key-values are bound to, sorted by the key so they can be binary searched
    let mut sorted_field_bindings = BTreeMap::new();
    // A shared counter used to generate unique idents
    let mut field_index = 0usize;

    let mut push_field_value = |k: String, mut fv: FieldValue| {
        let mut attrs = vec![];
        let mut cfg_attr = None;

        for attr in mem::take(&mut fv.attrs) {
            if attr.is_cfg() {
                assert!(
                    cfg_attr.is_none(),
                    "only a single #[cfg] is supported on fields"
                );
                cfg_attr = Some(attr);
            } else {
                attrs.push(attr);
            }
        }

        let v = Ident::new(&format!("__tmp{}", field_index), fv.span());

        field_values.push(
            quote_spanned!(fv.span()=> #cfg_attr { #(#attrs)* emit::ct::__private_capture!(#fv) }),
        );

        // If there's a #[cfg] then also push its reverse
        // This is to give a dummy value to the pattern binding since they don't support attributes
        if let Some(cfg_attr) = &cfg_attr {
            let cfg_attr = cfg_attr.invert_cfg().expect("attribute is not a #[cfg]");

            field_values.push(quote_spanned!(fv.span()=> #cfg_attr ()));
        }

        field_bindings.push(v.clone());

        // Make sure keys aren't duplicated
        let previous = sorted_field_bindings.insert(
            k.clone(),
            (
                quote_spanned!(fv.span()=> #cfg_attr #k),
                quote_spanned!(fv.span()=> #cfg_attr #v.clone()),
                quote_spanned!(fv.span()=> #cfg_attr &#v),
                cfg_attr,
            ),
        );
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
    let target_tokens = template
        .before_template_field_values()
        .find(|fv| {
            fv.key_name()
                .map(|k| k.as_str() == "target")
                .unwrap_or(false)
        })
        .map(|fv| {
            let target = &fv.expr;
            quote!(Some(#target))
        })
        .unwrap_or_else(|| quote!(None));

    // A runtime representation of the template
    let template_tokens = template.to_rt_tokens_with_visitor(
        quote!(emit::rt::__private),
        CfgVisitor(|label: &str| {
            sorted_field_bindings
                .get(label)
                .and_then(|(_, _, _, cfg_attr)| cfg_attr.as_ref())
        }),
    );

    let field_value_tokens = field_values.iter();
    let field_binding_tokens = field_bindings.iter();

    let sorted_field_binding_tokens = sorted_field_bindings.values().map(|(_, value, _, _)| value);
    let sorted_field_key_tokens = sorted_field_bindings.values().map(|(key, _, _, _)| key);
    let sorted_field_accessor_tokens = sorted_field_bindings.values().map(|(_, _, value, _)| value);
    let sorted_field_cfg_tokens = sorted_field_bindings.values().map(|(_, _, _, cfg_attr)| {
        if let Some(cfg_attr) = cfg_attr {
            quote!(#cfg_attr)
        }
        // If there isn't a cfg attribute then generate a dummy one
        // This attribute is always truthy so is ignored
        else {
            quote!(#[cfg(not(emit_rt__private_false))])
        }
    });

    quote!({
        match (#(#field_value_tokens),*) {
            (#(#field_binding_tokens),*) => {
                let kvs = emit::rt::__private::KeyValues {
                    sorted_key_values: &[#(#sorted_field_binding_tokens),*]
                };

                let template = #template_tokens;

                let #record_ident = emit::rt::__private::Record {
                    kvs,
                    template,
                };

                emit::rt::__private_forward!({
                    target: #target_tokens,
                    key_value_cfgs: [#(#sorted_field_cfg_tokens),*],
                    keys: [#(#sorted_field_key_tokens),*],
                    values: [#(#sorted_field_accessor_tokens),*],
                    record: &record,
                });
            }
        }
    })
}

struct CfgVisitor<F>(F);

impl<'a, F> fv_template::ct::Visitor for CfgVisitor<F>
where
    F: Fn(&str) -> Option<&'a Attribute> + 'a,
{
    fn visit_hole(&mut self, label: &str, hole: TokenStream) -> TokenStream {
        match (self.0)(label) {
            Some(cfg_attr) => {
                quote!(#cfg_attr #hole)
            }
            _ => hole,
        }
    }
}

pub(super) trait AttributeExt {
    fn is_cfg(&self) -> bool;
    fn invert_cfg(&self) -> Option<Attribute>;
}

impl AttributeExt for Attribute {
    fn is_cfg(&self) -> bool {
        if let Some(ident) = self.path.get_ident() {
            ident == "cfg"
        } else {
            false
        }
    }

    fn invert_cfg(&self) -> Option<Attribute> {
        match self.path.get_ident() {
            Some(ident) if ident == "cfg" => match self.parse_meta() {
                Ok(syn::Meta::List(list)) => {
                    let inner = list.nested;

                    Some(Attribute {
                        pound_token: self.pound_token.clone(),
                        style: self.style.clone(),
                        bracket_token: self.bracket_token.clone(),
                        path: self.path.clone(),
                        tokens: quote!((not(#inner))),
                    })
                }
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn expand_emit() {
        let cases = vec![
            (
                quote!("Text and {b: 17} and {a} and {#[with_debug] c} and {d: String::from(\"short lived\")} and {#[cfg(disabled)] e}"),
                quote!({
                    match (
                        {emit::ct::__private_capture!(b: 17) },
                        {emit::ct::__private_capture!(a) },
                        {
                            #[with_debug]
                            emit::ct::__private_capture!(c)
                        },
                        {emit::ct::__private_capture!(d: String::from("short lived")) },
                        #[cfg(disabled)]
                        {emit::ct::__private_capture!(e) },
                        #[cfg(not(disabled))]
                        ()
                    ) {
                        (__tmp0, __tmp1, __tmp2, __tmp3, __tmp4) => {
                            let kvs = emit::rt::__private::KeyValues {
                                sorted_key_values: &[
                                    __tmp1.clone(),
                                    __tmp0.clone(),
                                    __tmp2.clone(),
                                    __tmp3.clone(),
                                    #[cfg(disabled)]
                                    __tmp4.clone()
                                ]
                            };

                            let template = emit::rt::__private::template(&[
                                emit::rt::__private::Part::Text("Text and "),
                                emit::rt::__private::Part::Hole ( "b"),
                                emit::rt::__private::Part::Text(" and "),
                                emit::rt::__private::Part::Hole ( "a"),
                                emit::rt::__private::Part::Text(" and "),
                                emit::rt::__private::Part::Hole ( "c" ),
                                emit::rt::__private::Part::Text(" and "),
                                emit::rt::__private::Part::Hole ( "d" ),
                                emit::rt::__private::Part::Text(" and "),
                                #[cfg(disabled)]
                                emit::rt::__private::Part::Hole ( "e" )
                            ]);

                            let record = emit::rt::__private::Record {
                                kvs,
                                template,
                            };

                            emit::rt::__private_forward!({
                                target: None,
                                key_value_cfgs: [
                                    #[cfg(not(emit_rt__private_false))],
                                    #[cfg(not(emit_rt__private_false))],
                                    #[cfg(not(emit_rt__private_false))],
                                    #[cfg(not(emit_rt__private_false))],
                                    #[cfg(disabled)]
                                ],
                                keys: ["a", "b", "c", "d", #[cfg(disabled)] "e"],
                                values: [&__tmp1, &__tmp0, &__tmp2, &__tmp3, #[cfg(disabled)] &__tmp4],
                                record: &record,
                            });
                        }
                    }
                }),
            ),
            (
                quote!(target: log, "Text and {a}", a: 42),
                quote!({
                    match (
                        { emit::ct::__private_capture!(a: 42) }
                    ) {
                        (__tmp0) => {
                            let kvs = emit::rt::__private::KeyValues {
                                sorted_key_values: &[__tmp0.clone()]
                            };

                            let template = emit::rt::__private::template(&[
                                emit::rt::__private::Part::Text("Text and "),
                                emit::rt::__private::Part::Hole ( "a")
                            ]);

                            let record = emit::rt::__private::Record {
                                kvs,
                                template,
                            };

                            emit::rt::__private_forward!({
                                target: Some(log),
                                key_value_cfgs: [
                                    #[cfg(not(emit_rt__private_false))]
                                ],
                                keys: ["a"],
                                values: [&__tmp0],
                                record: &record,
                            });
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
