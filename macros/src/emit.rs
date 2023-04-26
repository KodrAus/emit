/*!
Compile-time implementation of event emission.

This module generates calls to `rt::emit`.
*/

use std::{collections::BTreeMap, mem};

use proc_macro2::{Span, TokenStream};
use syn::{spanned::Spanned, Attribute, Expr, ExprLit, ExprPath, FieldValue, Ident};

use fv_template::ct::Template;

use crate::{
    args::{self, Arg},
    util::{AttributeCfg, FieldValueKey},
};

pub(super) struct ExpandTokens {
    pub(super) receiver: TokenStream,
    pub(super) level: TokenStream,
    pub(super) input: TokenStream,
}

struct Args {
    to: TokenStream,
    when: TokenStream,
    with: TokenStream,
    ts: TokenStream,
}

impl Args {
    fn from_field_values<'a>(
        args: impl Iterator<Item = &'a FieldValue> + 'a,
    ) -> Result<Self, syn::Error> {
        let mut to = Arg::token_stream("to", |expr| Ok(quote!(Some(#expr))));
        let mut when = Arg::token_stream("when", |expr| Ok(quote!(Some(#expr))));
        let mut with = Arg::token_stream("with", |expr| Ok(quote!(Some(#expr))));
        let mut ts = Arg::token_stream("ts", |expr| Ok(quote!(Some(#expr))));

        args::set_from_field_values(args, [&mut to, &mut when, &mut with, &mut ts])?;

        Ok(Args {
            to: to.take().unwrap_or_else(|| quote!(emit::target::default())),
            when: when
                .take()
                .unwrap_or_else(|| quote!(emit::filter::default())),
            with: with.take().unwrap_or_else(|| quote!(emit::ctxt::default())),
            ts: ts.take().unwrap_or_else(|| quote!(None)),
        })
    }
}

pub(super) fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
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
    let args = Args::from_field_values(template.before_template_field_values())?;

    // A runtime representation of the template
    let template_tokens = {
        let mut template_visitor = TemplateVisitor {
            get_cfg_attr: |label: &str| {
                fields
                    .sorted_fields
                    .get(label)
                    .and_then(|field| field.cfg_attr.as_ref())
            },
            parts: Vec::new(),
        };
        template.visit(&mut template_visitor);
        let template_parts = &template_visitor.parts;

        quote!(emit::Template::new(&[
            #(#template_parts),*
        ]))
    };

    let field_match_value_tokens = fields.match_value_tokens();
    let field_match_binding_tokens = fields.match_binding_tokens();

    let field_event_tokens = fields.sorted_field_event_tokens();

    let to_tokens = args.to;
    let when_tokens = args.when;
    let with_tokens = args.with;
    let ts_tokens = args.ts;

    let level_tokens = {
        let level = opts.level;
        quote!(emit::Level::#level)
    };

    let receiver_tokens = opts.receiver;

    Ok(quote!({
        extern crate emit;

        match (#(#field_match_value_tokens),*) {
            (#(#field_match_binding_tokens),*) => {
                emit::#receiver_tokens(
                    #to_tokens,
                    #when_tokens,
                    #with_tokens,
                    #level_tokens,
                    #ts_tokens,
                    #template_tokens,
                    &[#(#field_event_tokens),*],
                )
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
    field_event_tokens: TokenStream,
    cfg_attr: Option<Attribute>,
}

impl Fields {
    fn match_value_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.match_value_tokens.iter()
    }

    fn match_binding_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.match_binding_tokens.iter()
    }

    fn sorted_field_event_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.sorted_fields
            .values()
            .map(|field| &field.field_event_tokens)
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
            quote_spanned!(fv.span()=> #cfg_attr { #(#attrs)* emit::__private_capture!(#fv) }),
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
                field_event_tokens: quote_spanned!(fv.span()=> #cfg_attr (emit::Key::new(#v.0), #v.1.by_ref())),
                cfg_attr,
            },
        );

        if previous.is_some() {
            return Err(syn::Error::new(fv.span(), "keys cannot be duplicated"));
        }

        Ok(())
    }
}

struct TemplateVisitor<F> {
    get_cfg_attr: F,
    parts: Vec<TokenStream>,
}

impl<'a, F> fv_template::ct::Visitor for TemplateVisitor<F>
where
    F: Fn(&str) -> Option<&'a Attribute> + 'a,
{
    fn visit_hole(&mut self, label: &str, hole: &ExprLit) {
        let hole = quote!({ emit::__private_fmt!(emit::template::Part::hole(#hole)) });

        match (self.get_cfg_attr)(label) {
            Some(cfg_attr) => self.parts.push(quote!(#cfg_attr #hole)),
            _ => self.parts.push(quote!(#hole)),
        }
    }

    fn visit_text(&mut self, text: &str) {
        self.parts.push(quote!(emit::template::Part::text(#text)));
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
                        {emit::__private_capture!(b: 17) },
                        {emit::__private_capture!(a) },
                        {
                            #[as_debug]
                            emit::__private_capture!(c)
                        },
                        {emit::__private_capture!(d: String::from("short lived")) },
                        #[cfg(disabled)]
                        {emit::__private_capture!(e) },
                        #[cfg(not(disabled))]
                        ()
                    ) {
                        (__tmp0, __tmp1, __tmp2, __tmp3, __tmp4) => {
                            let event = emit::RawEvent {
                                ts: emit::Timestamp::now(),
                                lvl: emit::Level::Info,
                                props: &[
                                    (__tmp1.0, __tmp1.1.by_ref()),
                                    (__tmp0.0, __tmp0.1.by_ref()),
                                    (__tmp2.0, __tmp2.1.by_ref()),
                                    (__tmp3.0, __tmp3.1.by_ref()),
                                    #[cfg(disabled)]
                                    (__tmp4.0, __tmp4.1.by_ref())
                                ],
                                tpl: emit::template::Template::new(&[
                                    emit::template::Part::Text("Text and "),
                                    emit::template::Part::Hole ( "b"),
                                    emit::template::Part::Text(" and "),
                                    emit::template::Part::Hole ( "a"),
                                    emit::template::Part::Text(" and "),
                                    emit::template::Part::Hole ( "c" ),
                                    emit::template::Part::Text(" and "),
                                    emit::template::Part::Hole ( "d" ),
                                    emit::template::Part::Text(" and "),
                                    #[cfg(disabled)]
                                    emit::template::Part::Hole ( "e" )
                                ]),
                            };

                            emit::__private_emit!({
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
                        { emit::__private_capture!(a: 42) }
                    ) {
                        (__tmp0) => {
                            let event = emit::Event {
                                ts: emit::Timestamp::now(),
                                lvl: emit::Level::Info,
                                props: &[(__tmp0.0, __tmp0.1.by_ref())],
                                tpl: emit::template::Template::new(&[
                                    emit::template::Part::Text("Text and "),
                                    emit::template::Part::Hole ( "a")
                                ]),
                            };

                            emit::__private_emit!({
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
                receiver: quote!(emit),
                level: quote!(Info),
                input: expr,
            }).unwrap();

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
