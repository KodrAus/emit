/*!
Compile-time implementation of event emission.

This module generates calls to `rt::emit`.
*/

use std::{collections::BTreeMap, mem};

use proc_macro2::{Span, TokenStream};
use syn::{
    spanned::Spanned, Attribute, Expr, ExprPath, FieldValue, Ident, MacroDelimiter, Meta, MetaList,
};

use fv_template::ct::Template;

use crate::capture::FieldValueExt;

pub(super) struct ExpandTokens {
    pub(super) receiver: TokenStream,
    pub(super) level: TokenStream,
    pub(super) input: TokenStream,
}

pub(super) fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let event_ident = Ident::new(&"event", opts.input.span());
    let template = Template::parse2(opts.input).map_err(|e| syn::Error::new(e.span(), e))?;

    // Any field-values that aren't part of the template
    let mut extra_field_values: BTreeMap<_, _> = template
        .after_template_field_values()
        .map(|fv| Ok((fv.key_name(), fv)))
        .collect::<Result<_, syn::Error>>()?;

    let mut fields = Fields::default();

    // Push the field-values that appear in the template
    for fv in template.template_field_values() {
        let k = fv.key_name();

        // If the hole has a corresponding field-value outside the template
        // then it will be used as the source for the value and attributes
        // In this case, it's expected that the field-value in the template is
        // just a single identifier
        let fv = match extra_field_values.remove(&k) {
            Some(extra_fv) => {
                if let Expr::Path(ExprPath { ref path, .. }) = fv.expr {
                    // Make sure the field-value in the template is just a plain identifier
                    if !fv.attrs.is_empty() {
                        return Err(syn::Error::new(fv.span(), "keys that exist in the template and extra pairs should only use attributes on the extra pair"));
                    }

                    assert_eq!(
                        path.get_ident().map(|ident| ident.to_string()).as_ref(),
                        Some(&k),
                        "the key name and path don't match"
                    );
                } else {
                    return Err(syn::Error::new(extra_fv.span(), "keys that exist in the template and extra pairs should only use identifiers"));
                }

                extra_fv
            }
            None => fv,
        };

        fields.push(k, fv.clone())?;
    }

    // Push any remaining extra field-values
    // This won't include any field values that also appear in the template
    for (k, fv) in extra_field_values {
        fields.push(k, fv.clone())?;
    }

    // Get the additional args to the log expression
    let args = Args::from_raw(template.before_template_field_values())?;

    // A runtime representation of the template
    let template_tokens = template.to_rt_tokens_with_visitor(
        quote!(emit::rt::__private),
        CfgVisitor(|label: &str| {
            fields
                .sorted_fields
                .get(label)
                .and_then(|field| field.cfg_attr.as_ref())
        }),
    );

    let field_match_value_tokens = fields.match_value_tokens();
    let field_match_binding_tokens = fields.match_binding_tokens();

    let field_event_tokens = fields.sorted_field_event_tokens();
    let field_cfg_tokens = fields.sorted_field_cfg_tokens();
    let field_key_tokens = fields.sorted_field_key_tokens();
    let field_value_tokens = fields.sorted_field_value_tokens();

    let to_tokens = args.to;
    let level_tokens = opts.level;
    let receiver_tokens = opts.receiver;

    Ok(quote!({
        extern crate emit;

        match (#(#field_match_value_tokens),*) {
            (#(#field_match_binding_tokens),*) => {
                let #event_ident = emit::rt::__private::RawEvent {
                    ts: emit::rt::__private::RawTimestamp::now(),
                    lvl: emit::rt::__private::RawLevel::#level_tokens,
                    props: &[#(#field_event_tokens),*],
                    tpl: #template_tokens,
                };

                emit::rt::#receiver_tokens!({
                    to: #to_tokens,
                    key_value_cfgs: [#(#field_cfg_tokens),*],
                    keys: [#(#field_key_tokens),*],
                    values: [#(#field_value_tokens),*],
                    event: &#event_ident,
                })
            }
        }
    }))
}

#[derive(Default)]
struct Fields {
    match_value_tokens: Vec<TokenStream>,
    match_binding_tokens: Vec<TokenStream>,
    sorted_fields: BTreeMap<String, SortedField>,
    field_index: usize,
}

struct SortedField {
    field_key_tokens: TokenStream,
    field_event_tokens: TokenStream,
    field_value_tokens: TokenStream,
    cfg_attr: Option<Attribute>,
}

impl Fields {
    fn match_value_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.match_value_tokens.iter()
    }

    fn match_binding_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.match_binding_tokens.iter()
    }

    fn sorted_field_key_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.sorted_fields
            .values()
            .map(|field| &field.field_key_tokens)
    }

    fn sorted_field_event_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.sorted_fields
            .values()
            .map(|field| &field.field_event_tokens)
    }

    fn sorted_field_value_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.sorted_fields
            .values()
            .map(|field| &field.field_value_tokens)
    }

    fn sorted_field_cfg_tokens(&'_ self) -> impl Iterator<Item = TokenStream> + '_ {
        self.sorted_fields.values().map(|field| {
            field
                .cfg_attr
                .as_ref()
                .map(|cfg_attr| quote!(#cfg_attr))
                .unwrap_or_else(|| quote!(#[cfg(not(emit_rt__private_false))]))
        })
    }

    fn next_ident(&mut self, span: Span) -> Ident {
        let i = Ident::new(&format!("__tmp{}", self.field_index), span);
        self.field_index += 1;

        i
    }

    fn push(&mut self, label: String, mut fv: FieldValue) -> Result<(), syn::Error> {
        let mut attrs = vec![];
        let mut cfg_attr = None;

        for attr in mem::take(&mut fv.attrs) {
            if attr.is_cfg() {
                if cfg_attr.is_some() {
                    return Err(syn::Error::new(
                        attr.span(),
                        "only a single #[cfg] is supported on fields",
                    ));
                }

                cfg_attr = Some(attr);
            } else {
                attrs.push(attr);
            }
        }

        let v = self.next_ident(fv.span());

        // NOTE: We intentionally wrap the expression in layers of blocks
        self.match_value_tokens.push(
            quote_spanned!(fv.span()=> #cfg_attr { #(#attrs)* emit::ct::__private_capture!(#fv) }),
        );

        // If there's a #[cfg] then also push its reverse
        // This is to give a dummy value to the pattern binding since they don't support attributes
        if let Some(cfg_attr) = &cfg_attr {
            let cfg_attr = cfg_attr
                .invert_cfg()
                .ok_or_else(|| syn::Error::new(cfg_attr.span(), "attribute is not a #[cfg]"))?;

            self.match_value_tokens
                .push(quote_spanned!(fv.span()=> #cfg_attr ()));
        }

        self.match_binding_tokens.push(quote!(#v));

        // Make sure keys aren't duplicated
        let previous = self.sorted_fields.insert(
            label.clone(),
            SortedField {
                field_key_tokens: quote_spanned!(fv.span()=> #cfg_attr #label),
                field_event_tokens: quote_spanned!(fv.span()=> #cfg_attr (#v.0, #v.1.by_ref())),
                field_value_tokens: quote_spanned!(fv.span()=> #cfg_attr &#v),
                cfg_attr,
            },
        );

        if previous.is_some() {
            return Err(syn::Error::new(fv.span(), "keys cannot be duplicated"));
        }

        Ok(())
    }
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
        if let Some(ident) = self.path().get_ident() {
            ident == "cfg"
        } else {
            false
        }
    }

    fn invert_cfg(&self) -> Option<Attribute> {
        match self.path().get_ident() {
            Some(ident) if ident == "cfg" => {
                let tokens = match &self.meta {
                    Meta::Path(meta) => quote!(not(#meta)),
                    Meta::List(meta) => {
                        let meta = &meta.tokens;
                        quote!(not(#meta))
                    }
                    Meta::NameValue(meta) => quote!(not(#meta)),
                };

                Some(Attribute {
                    pound_token: self.pound_token.clone(),
                    style: self.style.clone(),
                    bracket_token: self.bracket_token.clone(),
                    meta: Meta::List(MetaList {
                        path: self.path().clone(),
                        delimiter: MacroDelimiter::Paren(Default::default()),
                        tokens,
                    }),
                })
            }
            _ => None,
        }
    }
}

struct Args {
    to: TokenStream,
}

impl Args {
    fn from_raw<'a>(args: impl Iterator<Item = &'a FieldValue> + 'a) -> Result<Self, syn::Error> {
        let mut to = quote!(None);

        // Don't accept any unrecognized field names
        for fv in args {
            match &*fv.key_name() {
                "to" => {
                    let expr = &fv.expr;
                    to = quote!(Some(#expr));
                }
                unknown => {
                    return Err(syn::Error::new(
                        fv.span(),
                        format_args!("unexpected field `{}`", unknown),
                    ))
                }
            }
        }

        Ok(Args { to })
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
                quote!("Text and {b: 17} and {a} and {#[as_debug] c} and {d: String::from(\"short lived\")} and {#[cfg(disabled)] e}"),
                quote!({
                    extern crate emit;

                    match (
                        {emit::ct::__private_capture!(b: 17) },
                        {emit::ct::__private_capture!(a) },
                        {
                            #[as_debug]
                            emit::ct::__private_capture!(c)
                        },
                        {emit::ct::__private_capture!(d: String::from("short lived")) },
                        #[cfg(disabled)]
                        {emit::ct::__private_capture!(e) },
                        #[cfg(not(disabled))]
                        ()
                    ) {
                        (__tmp0, __tmp1, __tmp2, __tmp3, __tmp4) => {
                            let event = emit::rt::__private::RawEvent {
                                ts: emit::rt::__private::RawTimestamp::now(),
                                lvl: emit::rt::__private::RawLevel::info(),
                                props: &[
                                    (__tmp1.0, __tmp1.1.by_ref()),
                                    (__tmp0.0, __tmp0.1.by_ref()),
                                    (__tmp2.0, __tmp2.1.by_ref()),
                                    (__tmp3.0, __tmp3.1.by_ref()),
                                    #[cfg(disabled)]
                                    (__tmp4.0, __tmp4.1.by_ref())
                                ],
                                tpl: emit::rt::__private::template(&[
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
                                ]),
                            };

                            emit::rt::__private_emit!({
                                to: None,
                                key_value_cfgs: [
                                    #[cfg(not(emit_rt__private_false))],
                                    #[cfg(not(emit_rt__private_false))],
                                    #[cfg(not(emit_rt__private_false))],
                                    #[cfg(not(emit_rt__private_false))],
                                    #[cfg(disabled)]
                                ],
                                keys: ["a", "b", "c", "d", #[cfg(disabled)] "e"],
                                values: [&__tmp1, &__tmp0, &__tmp2, &__tmp3, #[cfg(disabled)] &__tmp4],
                                event: &event,
                            })
                        }
                    }
                }),
            ),
            (
                quote!(to: log, "Text and {a}", a: 42),
                quote!({
                    extern crate emit;

                    match (
                        { emit::ct::__private_capture!(a: 42) }
                    ) {
                        (__tmp0) => {
                            let event = emit::rt::__private::RawEvent {
                                ts: emit::rt::__private::RawTimestamp::now(),
                                lvl: emit::rt::__private::RawLevel::info(),
                                props: &[(__tmp0.0, __tmp0.1.by_ref())],
                                tpl: emit::rt::__private::template(&[
                                    emit::rt::__private::Part::Text("Text and "),
                                    emit::rt::__private::Part::Hole ( "a")
                                ]),
                            };

                            emit::rt::__private_emit!({
                                to: Some(log),
                                key_value_cfgs: [
                                    #[cfg(not(emit_rt__private_false))]
                                ],
                                keys: ["a"],
                                values: [&__tmp0],
                                event: &event,
                            })
                        }
                    }
                })
            )
        ];

        for (expr, expected) in cases {
            let actual = expand_tokens(ExpandTokens {
                receiver: quote!(__private_emit),
                level: quote!(info()),
                input: expr,
            }).unwrap();

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
