use std::collections::BTreeMap;

use proc_macro2::{Span, TokenStream};
use syn::{spanned::Spanned, Attribute, FieldValue, Ident};

use crate::{capture, util::AttributeCfg};

#[derive(Default)]
pub(super) struct Props {
    match_value_tokens: Vec<TokenStream>,
    match_binding_tokens: Vec<TokenStream>,
    sorted_fields: BTreeMap<String, Field>,
    field_index: usize,
}

pub(super) struct Field {
    field_event_tokens: TokenStream,
    pub cfg_attr: Option<Attribute>,
    pub attrs: Vec<Attribute>,
}

impl Props {
    pub(super) fn match_value_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.match_value_tokens.iter()
    }

    pub(super) fn match_binding_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.match_binding_tokens.iter()
    }

    pub(super) fn sorted_key_value_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.sorted_fields
            .values()
            .map(|field| &field.field_event_tokens)
    }

    fn next_ident(&mut self, span: Span) -> Ident {
        let i = Ident::new(&format!("__tmp{}", self.field_index), span);
        self.field_index += 1;

        i
    }

    pub(super) fn get(&self, label: &str) -> Option<&Field> {
        self.sorted_fields.get(label)
    }

    pub(super) fn push(&mut self, label: String, fv: &FieldValue) -> Result<(), syn::Error> {
        let mut attrs = vec![];
        let mut cfg_attr = None;

        for attr in &fv.attrs {
            if attr.is_cfg() {
                if cfg_attr.is_some() {
                    return Err(syn::Error::new(
                        attr.span(),
                        "only a single #[cfg] is supported on fields",
                    ));
                }

                cfg_attr = Some(attr.clone());
            } else {
                attrs.push(attr.clone());
            }
        }

        let v = self.next_ident(fv.span());

        let capture_tokens = capture::key_value_with_hook(&attrs, &fv);

        self.match_value_tokens.push(match cfg_attr {
            Some(ref cfg_attr) => quote_spanned!(fv.span()=>
                #cfg_attr
                {
                    #capture_tokens
                }
            ),
            None => capture_tokens,
        });

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
            Field {
                field_event_tokens: quote_spanned!(fv.span()=> #cfg_attr (emit::Key::new(#v.0), #v.1.by_ref())),
                cfg_attr,
                attrs,
            },
        );

        if previous.is_some() {
            return Err(syn::Error::new(fv.span(), "keys cannot be duplicated"));
        }

        Ok(())
    }
}
