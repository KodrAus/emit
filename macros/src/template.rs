use std::collections::BTreeMap;

use proc_macro2::TokenStream;

use syn::{
    parse::Parse, punctuated::Punctuated, spanned::Spanned, Expr, ExprLit, ExprPath, FieldValue,
};

use crate::{fmt, props::Props, util::FieldValueKey};

pub(super) fn parse2<A: Parse>(input: TokenStream) -> Result<(A, Template, Props), syn::Error> {
    let template =
        fv_template::ct::Template::parse2(input).map_err(|e| syn::Error::new(e.span(), e))?;

    // Parse args from the field values before the template
    let args = {
        let args = template
            .before_template_field_values()
            .cloned()
            .collect::<Punctuated<FieldValue, Token![,]>>();
        syn::parse2(quote!(#args))?
    };

    // Any field-values that aren't part of the template
    let mut extra_field_values: BTreeMap<_, _> = template
        .after_template_field_values()
        .map(|fv| Ok((fv.key_name(), fv)))
        .collect::<Result<_, syn::Error>>()?;

    let mut props = Props::default();

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

        props.push(k, fv)?;
    }

    // Push any remaining extra field-values
    // This won't include any field values that also appear in the template
    for (k, fv) in extra_field_values {
        props.push(k, fv)?;
    }

    // A runtime representation of the template
    let template_tokens = {
        let mut template_visitor = TemplateVisitor {
            props: &props,
            parts: Vec::new(),
        };
        template.visit(&mut template_visitor);
        let template_parts = &template_visitor.parts;

        quote!(emit::Template::new_ref(&[
            #((#template_parts)),*
        ]))
    };

    Ok((args, Template { template_tokens }, props))
}

pub(super) struct Template {
    template_tokens: TokenStream,
}

impl Template {
    pub fn template_tokens(&self) -> &TokenStream {
        &self.template_tokens
    }
}

struct TemplateVisitor<'a> {
    props: &'a Props,
    parts: Vec<TokenStream>,
}

impl<'a> fv_template::ct::Visitor for TemplateVisitor<'a> {
    fn visit_hole(&mut self, label: &str, hole: &ExprLit) {
        let field = self.props.get(label).expect("missing prop");

        let hole_tokens = fmt::template_hole_with_hook(&field.attrs, hole);

        match field.cfg_attr {
            Some(ref cfg_attr) => self.parts.push(quote!(#cfg_attr { #hole_tokens })),
            _ => self.parts.push(quote!(#hole_tokens)),
        }
    }

    fn visit_text(&mut self, text: &str) {
        self.parts.push(quote!(emit::template::Part::text(#text)));
    }
}
