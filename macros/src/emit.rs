use proc_macro2::TokenStream;
use syn::{parse::Parse, FieldValue};

use crate::{
    args::{self, Arg},
    event::push_event_props,
    template,
};

pub struct ExpandTokens {
    pub level: Option<TokenStream>,
    pub input: TokenStream,
}

struct Args {
    rt: TokenStream,
    extent: TokenStream,
    to: TokenStream,
    when: TokenStream,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut extent = Arg::token_stream("extent", |fv| {
            let expr = &fv.expr;

            Ok(quote!(#expr))
        });
        let mut rt = Arg::token_stream("rt", |fv| {
            let expr = &fv.expr;

            Ok(quote!(#expr))
        });
        let mut to = Arg::token_stream("to", |fv| {
            let expr = &fv.expr;

            Ok(quote!(#expr))
        });
        let mut when = Arg::token_stream("when", |fv| {
            let expr = &fv.expr;

            Ok(quote!(#expr))
        });

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut extent, &mut rt, &mut to, &mut when],
        )?;

        Ok(Args {
            extent: extent.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
            rt: rt.take().unwrap_or_else(|| quote!(emit::runtime::shared())),
            to: to.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
            when: when.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let (args, template, mut props) = template::parse2::<Args>(opts.input, true)?;

    push_event_props(&mut props, opts.level)?;

    let props_match_input_tokens = props.match_input_tokens();
    let props_match_binding_tokens = props.match_binding_tokens();
    let props_tokens = props.match_bound_tokens();

    let extent_tokens = args.extent;
    let rt_tokens = args.rt;
    let to_tokens = args.to;
    let when_tokens = args.when;

    let template_tokens = template.template_tokens();

    Ok(quote!({
        match (#(#props_match_input_tokens),*) {
            (#(#props_match_binding_tokens),*) => {
                emit::__private::__private_emit(
                    #rt_tokens,
                    #to_tokens,
                    #when_tokens,
                    #extent_tokens,
                    #template_tokens,
                    #props_tokens,
                )
            }
        }
    }))
}
