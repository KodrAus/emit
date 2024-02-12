use proc_macro2::{Ident, Span, TokenStream};
use syn::{
    parse::Parse, spanned::Spanned, Block, Expr, ExprAsync, ExprBlock, FieldValue, Item, ItemFn,
    Signature, Stmt,
};

use crate::{
    args::{self, Arg},
    event::push_event_props,
    props::Props,
    template::{self, Template},
};

pub struct ExpandTokens {
    pub level: Option<TokenStream>,
    pub item: TokenStream,
    pub input: TokenStream,
}

struct Args {
    rt: TokenStream,
    to: TokenStream,
    when: TokenStream,
    arg: Option<Ident>,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
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
        let mut arg = Arg::ident("arg");

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut arg, &mut rt, &mut to, &mut when],
        )?;

        Ok(Args {
            rt: rt.take().unwrap_or_else(|| quote!(emit::runtime::shared())),
            to: to.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
            when: when.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
            arg: arg.take(),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let (args, template, ctxt_props) = template::parse2::<Args>(opts.input, true)?;

    let mut evt_props = Props::new();
    push_event_props(&mut evt_props, opts.level)?;

    let span_arg = args
        .arg
        .unwrap_or_else(|| Ident::new("__span", Span::call_site()));

    let mut item = syn::parse2::<Stmt>(opts.item)?;
    match &mut item {
        // A synchronous function
        Stmt::Item(Item::Fn(ItemFn {
            block,
            sig: Signature {
                asyncness: None, ..
            },
            ..
        })) => {
            **block = syn::parse2::<Block>(inject_sync(
                &args.rt,
                &args.to,
                &args.when,
                &template,
                &ctxt_props,
                &evt_props,
                &span_arg,
                quote!(#block),
            ))?;
        }
        // A synchronous block
        Stmt::Expr(Expr::Block(ExprBlock { block, .. }), _) => {
            *block = syn::parse2::<Block>(inject_sync(
                &args.rt,
                &args.to,
                &args.when,
                &template,
                &ctxt_props,
                &evt_props,
                &span_arg,
                quote!(#block),
            ))?;
        }
        // An asynchronous function
        Stmt::Item(Item::Fn(ItemFn {
            block,
            sig: Signature {
                asyncness: Some(_), ..
            },
            ..
        })) => {
            **block = syn::parse2::<Block>(inject_async(
                &args.rt,
                &args.to,
                &args.when,
                &template,
                &ctxt_props,
                &evt_props,
                &span_arg,
                quote!(#block),
            ))?;
        }
        // An asynchronous block
        Stmt::Expr(Expr::Async(ExprAsync { block, .. }), _) => {
            *block = syn::parse2::<Block>(inject_async(
                &args.rt,
                &args.to,
                &args.when,
                &template,
                &ctxt_props,
                &evt_props,
                &span_arg,
                quote!(#block),
            ))?;
        }
        _ => return Err(syn::Error::new(item.span(), "unrecognized item type")),
    }

    Ok(quote!(#item))
}

fn inject_sync(
    rt_tokens: &TokenStream,
    to_tokens: &TokenStream,
    when_tokens: &TokenStream,
    template: &Template,
    ctxt_props: &Props,
    evt_props: &Props,
    span_arg: &Ident,
    body: TokenStream,
) -> TokenStream {
    let ctxt_props_tokens = ctxt_props.props_tokens();
    let evt_props_tokens = evt_props.props_tokens();
    let template_tokens = template.template_tokens();

    quote!({
        let (mut __ctxt, __timer) = emit::__private::__private_push_span_ctxt(#rt_tokens, #when_tokens, #template_tokens, #ctxt_props_tokens, #evt_props_tokens);
        let __ctxt_guard = __ctxt.enter();

        let #span_arg = emit::__private::__private_begin_span(__timer, |extent| {
            emit::__private::__private_emit(
                #rt_tokens,
                #to_tokens,
                emit::empty::Empty,
                extent,
                #template_tokens,
                #evt_props_tokens,
            )
        });

        #body
    })
}

fn inject_async(
    rt_tokens: &TokenStream,
    to_tokens: &TokenStream,
    when_tokens: &TokenStream,
    template: &Template,
    ctxt_props: &Props,
    evt_props: &Props,
    span_arg: &Ident,
    body: TokenStream,
) -> TokenStream {
    let ctxt_props_tokens = ctxt_props.props_tokens();
    let evt_props_tokens = evt_props.props_tokens();
    let template_tokens = template.template_tokens();

    quote!({
        let (__ctxt, __timer) = emit::__private::__private_push_span_ctxt(#rt_tokens, #when_tokens, #template_tokens, #ctxt_props_tokens, #evt_props_tokens);

        __ctxt.with_future(async {
            let #span_arg = emit::__private::__private_begin_span(__timer, |extent| {
                emit::__private::__private_emit(
                    #rt_tokens,
                    #to_tokens,
                    emit::empty::Empty,
                    extent,
                    #template_tokens,
                    #evt_props_tokens,
                )
            });

            async #body.await
        }).await
    })
}
