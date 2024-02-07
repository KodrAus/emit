use proc_macro2::TokenStream;
use syn::{spanned::Spanned, Block, Expr, ExprAsync, ExprBlock, Item, ItemFn, Signature, Stmt};

use crate::props::Props;

pub struct ExpandTokens {
    pub item: TokenStream,
    pub input: TokenStream,
    pub root: bool,
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let props = syn::parse2::<Props>(opts.input)?;

    let target = if opts.root {
        quote!(emit::__private::__private_root_ctxt)
    } else {
        quote!(emit::__private::__private_push_ctxt)
    };

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
            **block = syn::parse2::<Block>(inject_sync(&target, &props, quote!(#block)))?;
        }
        // A synchronous block
        Stmt::Expr(Expr::Block(ExprBlock { block, .. }), _) => {
            *block = syn::parse2::<Block>(inject_sync(&target, &props, quote!(#block)))?;
        }
        // An asynchronous function
        Stmt::Item(Item::Fn(ItemFn {
            block,
            sig: Signature {
                asyncness: Some(_), ..
            },
            ..
        })) => {
            **block = syn::parse2::<Block>(inject_async(&target, &props, quote!(#block)))?;
        }
        // An asynchronous block
        Stmt::Expr(Expr::Async(ExprAsync { block, .. }), _) => {
            *block = syn::parse2::<Block>(inject_async(&target, &props, quote!(#block)))?;
        }
        _ => return Err(syn::Error::new(item.span(), "unrecognized item type")),
    }

    Ok(quote!(#item))
}

fn inject_sync(target: &TokenStream, props: &Props, body: TokenStream) -> TokenStream {
    let props_tokens = props.props_tokens();

    quote!({
        let mut __ctxt = #target(#props_tokens);
        let __ctxt_guard = __ctxt.enter();

        #body
    })
}

fn inject_async(target: &TokenStream, props: &Props, body: TokenStream) -> TokenStream {
    let props_tokens = props.props_tokens();

    quote!({
        let __ctxt = #target(#props_tokens);
        __ctxt.with_future(async #body).await
    })
}
