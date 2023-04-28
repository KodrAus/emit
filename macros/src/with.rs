use proc_macro2::TokenStream;
use syn::{parse::Parse, spanned::Spanned, Block, FieldValue, Item, ItemFn, Signature};

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
    linker: TokenStream,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut linker = Arg::token_stream("linker", |expr| Ok(quote!(#expr)));

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut linker],
        )?;

        Ok(Args {
            linker: linker
                .take()
                .unwrap_or_else(|| quote!(emit::global_linker())),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let (args, _, props) = template::parse2::<Args>(opts.input)?;

    let linker_tokens = args.linker;

    let mut item = syn::parse2::<Item>(opts.item)?;
    match &mut item {
        // A synchronous function
        Item::Fn(ItemFn {
            block,
            sig: Signature {
                asyncness: None, ..
            },
            ..
        }) => {
            **block = syn::parse2::<Block>(inject_sync(&props, linker_tokens, quote!(#block)))?;
        }
        // An asynchronous function
        Item::Fn(ItemFn {
            block,
            sig: Signature {
                asyncness: Some(_), ..
            },
            ..
        }) => {
            **block = syn::parse2::<Block>(inject_async(&props, linker_tokens, quote!(#block)))?;
        }
        _ => return Err(syn::Error::new(item.span(), "unrecognized item type")),
    }

    Ok(quote!(#item))
}

fn inject_sync(props: &Props, linker_tokens: TokenStream, body: TokenStream) -> TokenStream {
    let props_tokens = props.props_tokens();

    quote!({
        let mut __link = emit::ctxt::Link::new(#linker_tokens, #props_tokens);
        let __link_guard = __link.link();

        #body
    })
}

fn inject_async(props: &Props, linker_tokens: TokenStream, body: TokenStream) -> TokenStream {
    let props_tokens = props.props_tokens();

    quote!({
        emit::ctxt::LinkFuture::new(#linker_tokens, #props_tokens, async #body).await
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
                        emit::global_linker(),
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
                        emit::global_linker(),
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
