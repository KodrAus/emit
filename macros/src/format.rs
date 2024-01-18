use proc_macro2::TokenStream;
use syn::{parse::Parse, FieldValue};

use crate::{
    args::{self},
    template,
};

pub struct ExpandTokens {
    pub input: TokenStream,
}

struct Args {}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [],
        )?;

        Ok(Args {})
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let (_, template, props) = template::parse2::<Args>(opts.input, true)?;

    let props_match_input_tokens = props.match_input_tokens();
    let props_match_binding_tokens = props.match_binding_tokens();
    let props_tokens = props.match_bound_tokens();

    let template_tokens = template.template_tokens();

    Ok(quote!({
        match (#(#props_match_input_tokens),*) {
            (#(#props_match_binding_tokens),*) => {
                emit::__private::__private_format(
                    #template_tokens,
                    #props_tokens,
                )
            }
        }
    }))
}
