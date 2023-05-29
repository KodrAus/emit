use proc_macro2::TokenStream;
use syn::{
    parse::Parse, spanned::Spanned, Block, Expr, ExprAsync, ExprBlock, FieldValue, Item, ItemFn,
    Signature, Stmt,
};

use crate::{
    args::{self, Arg},
    props::Props,
    template,
};

pub struct ExpandTokens {
    pub item: TokenStream,
    pub input: TokenStream,
}

struct Args {
    with: TokenStream,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut with = Arg::token_stream("with", |expr| Ok(quote!(#expr)));

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut with],
        )?;

        Ok(Args {
            with: with.take().unwrap_or_else(|| quote!(emit::ctxt())),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let (args, template, props) = template::parse2::<Args>(opts.input)?;

    let template_tokens = template.template_tokens();

    let with_tokens = args.with;

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
                &props,
                template_tokens,
                with_tokens,
                quote!(#block),
            ))?;
        }
        // A synchronous block
        Stmt::Expr(Expr::Block(ExprBlock { block, .. }), _) => {
            *block = syn::parse2::<Block>(inject_sync(
                &props,
                template_tokens,
                with_tokens,
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
                &props,
                template_tokens,
                with_tokens,
                quote!(#block),
            ))?;
        }
        // An asynchronous block
        Stmt::Expr(Expr::Async(ExprAsync { block, .. }), _) => {
            *block = syn::parse2::<Block>(inject_async(
                &props,
                template_tokens,
                with_tokens,
                quote!(#block),
            ))?;
        }
        _ => return Err(syn::Error::new(item.span(), "unrecognized item type")),
    }

    Ok(quote!(#item))
}

fn inject_sync(
    props: &Props,
    template_tokens: TokenStream,
    with_tokens: TokenStream,
    body: TokenStream,
) -> TokenStream {
    let props_tokens = props.props_tokens();

    quote!({
        let mut __span = emit::span(#with_tokens, emit::Id::EMPTY, #template_tokens, #props_tokens);
        let __span_guard = __span.enter();

        #body
    })
}

fn inject_async(
    props: &Props,
    template_tokens: TokenStream,
    with_tokens: TokenStream,
    body: TokenStream,
) -> TokenStream {
    let props_tokens = props.props_tokens();

    quote!({
        emit::span_future(#with_tokens, emit::Id::EMPTY, #template_tokens, #props_tokens, async #body).await
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
