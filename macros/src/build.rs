use proc_macro2::TokenStream;
use syn::{parse::Parse, FieldValue};

use crate::{args, props::Props, template};

pub struct ExpandPropsTokens {
    pub input: TokenStream,
}

pub fn expand_props_tokens(opts: ExpandPropsTokens) -> Result<TokenStream, syn::Error> {
    let props = syn::parse2::<Props>(opts.input)?;

    Ok(props.props_tokens())
}

pub struct ExpandTemplateTokens {
    pub input: TokenStream,
}

pub struct TemplateArgs {}

impl Parse for TemplateArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [],
        )?;

        Ok(TemplateArgs {})
    }
}

pub fn expand_template_tokens(opts: ExpandTemplateTokens) -> Result<TokenStream, syn::Error> {
    let (_, template, props) = template::parse2::<TemplateArgs>(opts.input)?;

    // Ensure that a standalone template only specifies identifiers
    for key_value in props.iter() {
        if key_value.has_expr {
            return Err(syn::Error::new(
                key_value.span(),
                "key-values in raw templates cannot capture values",
            ));
        }
    }

    Ok(template.template_tokens())
}
