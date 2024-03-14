use proc_macro2::TokenStream;
use syn::{parse::Parse, FieldValue};

use crate::{
    args::{self, Arg},
    event::push_event_props,
    module::module_tokens,
    template,
};

pub struct ExpandTokens {
    pub level: Option<TokenStream>,
    pub input: TokenStream,
}

struct Args {
    rt: TokenStream,
    module: TokenStream,
    props: TokenStream,
    extent: TokenStream,
    when: TokenStream,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut module = Arg::token_stream("module", |fv| {
            let expr = &fv.expr;

            Ok(quote!(#expr))
        });
        let mut extent = Arg::token_stream("extent", |fv| {
            let expr = &fv.expr;

            Ok(quote!(#expr))
        });
        let mut rt = Arg::token_stream("rt", |fv| {
            let expr = &fv.expr;

            Ok(quote!(#expr))
        });
        let mut props = Arg::token_stream("props", |fv| {
            let expr = &fv.expr;

            Ok(quote!(#expr))
        });
        let mut when = Arg::token_stream("when", |fv| {
            let expr = &fv.expr;

            Ok(quote!(#expr))
        });

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut module, &mut extent, &mut props, &mut rt, &mut when],
        )?;

        Ok(Args {
            module: module.take().unwrap_or_else(|| module_tokens()),
            extent: extent.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
            props: props.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
            rt: rt.take_rt()?,
            when: when.take_when(),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let (args, template, mut props) = template::parse2::<Args>(opts.input, true)?;

    push_event_props(&mut props, opts.level)?;

    let props_match_input_tokens = props.match_input_tokens();
    let props_match_binding_tokens = props.match_binding_tokens();
    let base_props_tokens = args.props;
    let props_tokens = props.match_bound_tokens();

    let extent_tokens = args.extent;
    let rt_tokens = args.rt;
    let when_tokens = args.when;
    let module_tokens = args.module;

    let template_tokens = template.template_tokens();

    Ok(quote!({
        match (#(#props_match_input_tokens),*) {
            (#(#props_match_binding_tokens),*) => {
                emit::__private::__private_emit(
                    #rt_tokens,
                    #module_tokens,
                    #when_tokens,
                    #extent_tokens,
                    #template_tokens,
                    emit::Props::chain(&#base_props_tokens, #props_tokens),
                )
            }
        }
    }))
}
