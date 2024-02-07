use proc_macro2::{Span, TokenStream};
use syn::{parse::Parse, FieldValue, Ident};

use crate::{
    args::{self, Arg},
    props::Props,
    template,
};

pub struct ExpandTokens {
    pub level: Option<TokenStream>,
    pub input: TokenStream,
}

struct Args {
    extent: TokenStream,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut extent = Arg::token_stream("extent", |fv| {
            let expr = &fv.expr;

            Ok(quote!(#expr))
        });

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut extent],
        )?;

        Ok(Args {
            extent: extent.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let (args, template, mut props) = template::parse2::<Args>(opts.input, true)?;

    push_event_props(&mut props, opts.level)?;

    let extent_tokens = args.extent;
    let template_tokens = template.template_tokens();
    let props_tokens = props.props_tokens();

    Ok(quote!(emit::Event::new(#extent_tokens, #template_tokens, #props_tokens)))
}

pub fn push_event_props(props: &mut Props, level: Option<TokenStream>) -> Result<(), syn::Error> {
    // Add the level as a property
    if let Some(level_value) = level {
        let level_ident = Ident::new(emit_core::well_known::LVL_KEY, Span::call_site());

        props.push(
            &syn::parse2::<FieldValue>(quote!(#level_ident: emit::Level::#level_value))?,
            false,
            true,
        )?;
    }

    // Add the location as a property
    let loc_ident = Ident::new(emit_core::well_known::MODULE_KEY, Span::call_site());
    props.push(
        &syn::parse2::<FieldValue>(quote!(#loc_ident: emit::__private::__private_module!()))?,
        false,
        true,
    )?;

    Ok(())
}
