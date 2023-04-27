use std::collections::BTreeMap;

use proc_macro2::{Span, TokenStream};
use syn::{parse::Parse, spanned::Spanned, Attribute, FieldValue, Ident};

use crate::{
    capture,
    util::{AttributeCfg, FieldValueKey},
};

#[derive(Default)]
pub struct Props {
    match_value_tokens: Vec<TokenStream>,
    match_binding_tokens: Vec<TokenStream>,
    key_values: BTreeMap<String, KeyValue>,
    key_value_index: usize,
}

impl Parse for Props {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let fv = input.parse_terminated(FieldValue::parse, Token![,])?;

        let mut props = Props::new();

        for fv in fv {
            props.push(&fv)?;
        }

        Ok(props)
    }
}

pub struct KeyValue {
    match_bound_tokens: TokenStream,
    direct_bound_tokens: TokenStream,
    span: Span,
    pub cfg_attr: Option<Attribute>,
    pub attrs: Vec<Attribute>,
    pub has_expr: bool,
}

impl KeyValue {
    pub fn span(&self) -> Span {
        self.span.clone()
    }
}

impl Props {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn match_input_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.match_value_tokens.iter()
    }

    pub fn match_binding_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.match_binding_tokens.iter()
    }

    pub fn match_bound_tokens(&self) -> TokenStream {
        Self::sorted_props_tokens(self.key_values.values().map(|kv| &kv.match_bound_tokens))
    }

    pub fn props_tokens(&self) -> TokenStream {
        Self::sorted_props_tokens(self.key_values.values().map(|kv| &kv.direct_bound_tokens))
    }

    fn sorted_props_tokens<'a>(
        key_values: impl Iterator<Item = &'a TokenStream> + 'a,
    ) -> TokenStream {
        quote!(emit::props::SortedSlice::new_ref(&[#(#key_values),*]))
    }

    fn next_match_binding_ident(&mut self, span: Span) -> Ident {
        let i = Ident::new(&format!("__tmp{}", self.key_value_index), span);
        self.key_value_index += 1;

        i
    }

    pub fn get(&self, label: &str) -> Option<&KeyValue> {
        self.key_values.get(label)
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a KeyValue> + 'a {
        self.key_values.values()
    }

    pub fn push(&mut self, fv: &FieldValue) -> Result<(), syn::Error> {
        let mut attrs = vec![];
        let mut cfg_attr = None;

        for attr in &fv.attrs {
            if attr.is_cfg() {
                if cfg_attr.is_some() {
                    return Err(syn::Error::new(
                        attr.span(),
                        "only a single #[cfg] is supported on key-value pairs",
                    ));
                }

                cfg_attr = Some(attr.clone());
            } else {
                attrs.push(attr.clone());
            }
        }

        let match_bound_ident = self.next_match_binding_ident(fv.span());

        let key_value_tokens = {
            let key_value_tokens = capture::key_value_with_hook(&attrs, &fv);

            match cfg_attr {
                Some(ref cfg_attr) => quote_spanned!(fv.span()=>
                    #cfg_attr
                    {
                        #key_value_tokens
                    }
                ),
                None => key_value_tokens,
            }
        };

        self.match_value_tokens.push(key_value_tokens.clone());

        // If there's a #[cfg] then also push its reverse
        // This is to give a dummy value to the pattern binding since they don't support attributes
        if let Some(cfg_attr) = &cfg_attr {
            let cfg_attr = cfg_attr
                .invert_cfg()
                .ok_or_else(|| syn::Error::new(cfg_attr.span(), "attribute is not a #[cfg]"))?;

            self.match_value_tokens
                .push(quote_spanned!(fv.span()=> #cfg_attr ()));
        }

        self.match_binding_tokens
            .push(quote_spanned!(fv.span()=> #match_bound_ident));

        // Make sure keys aren't duplicated
        let previous = self.key_values.insert(
            fv.key_name(),
            KeyValue {
                match_bound_tokens: quote_spanned!(fv.span()=> #cfg_attr (#match_bound_ident.0.by_ref(), #match_bound_ident.1.by_ref())),
                direct_bound_tokens: quote_spanned!(fv.span()=> #key_value_tokens),
                span: fv.span(),
                has_expr: fv.colon_token.is_some(),
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
