use proc_macro2::TokenStream;
use syn::{
    parse::Parse, spanned::Spanned, Block, Expr, ExprAsync, ExprBlock, Item, ItemFn, Signature,
    Stmt,
};

use crate::props::Props;

pub struct ExpandTokens {
    pub sync_receiver: TokenStream,
    pub async_receiver: TokenStream,
    pub item: TokenStream,
    pub input: TokenStream,
}

struct Args {}

impl Parse for Args {
    fn parse(_: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Args {})
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let props = syn::parse2::<Props>(opts.input)?;

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
            **block =
                syn::parse2::<Block>(inject_sync(&props, opts.sync_receiver, quote!(#block)))?;
        }
        // A synchronous block
        Stmt::Expr(Expr::Block(ExprBlock { block, .. }), _) => {
            *block = syn::parse2::<Block>(inject_sync(&props, opts.sync_receiver, quote!(#block)))?;
        }
        // An asynchronous function
        Stmt::Item(Item::Fn(ItemFn {
            block,
            sig: Signature {
                asyncness: Some(_), ..
            },
            ..
        })) => {
            **block =
                syn::parse2::<Block>(inject_async(&props, opts.async_receiver, quote!(#block)))?;
        }
        // An asynchronous block
        Stmt::Expr(Expr::Async(ExprAsync { block, .. }), _) => {
            *block =
                syn::parse2::<Block>(inject_async(&props, opts.async_receiver, quote!(#block)))?;
        }
        _ => return Err(syn::Error::new(item.span(), "unrecognized item type")),
    }

    Ok(quote!(#item))
}

fn inject_sync(props: &Props, receiver_tokens: TokenStream, body: TokenStream) -> TokenStream {
    let props_tokens = props.props_tokens();

    quote!({
        let mut __ctxt = emit::#receiver_tokens(#props_tokens);
        let __ctxt_guard = __ctxt.enter();

        #body
    })
}

fn inject_async(props: &Props, receiver_tokens: TokenStream, body: TokenStream) -> TokenStream {
    let props_tokens = props.props_tokens();

    quote!({
        let __ctxt = emit::#receiver_tokens(#props_tokens);
        __ctxt.into_future(async #body).await
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_fn_sync() {
        for (args, item, expected) in [(
            quote!("{a}"),
            quote!(
                fn some_fn(a: i32) {
                    a + 1;
                }
            ),
            quote!(
                fn some_fn(a: i32) {
                    let mut __link = emit::ctxt::Link::new(
                        emit::ambient_with(),
                        emit::props::SortedSlice::new_ref(&[{
                            use emit::__private::__PrivateCaptureHook;
                            (emit::Key::new("a"), (a).__private_capture_as_default())
                        }]),
                    );
                    let __link_guard = __link.link();

                    {
                        a + 1;
                    }
                }
            ),
        )] {
            let actual = expand_tokens(ExpandTokens { input: args, item }).unwrap();

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }

    #[test]
    fn inject_fn_async() {
        for (args, item, expected) in [(
            quote!("{a}"),
            quote!(
                async fn some_fn(a: i32) {
                    a + 1;
                }
            ),
            quote!(
                async fn some_fn(a: i32) {
                    emit::ctxt::LinkFuture::new(
                        emit::ambient_with(),
                        emit::props::SortedSlice::new_ref(&[{
                            use emit::__private::__PrivateCaptureHook;
                            (emit::Key::new("a"), (a).__private_capture_as_default())
                        }]),
                        async {
                            a + 1;
                        },
                    )
                    .await
                }
            ),
        )] {
            let actual = expand_tokens(ExpandTokens { input: args, item }).unwrap();

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
