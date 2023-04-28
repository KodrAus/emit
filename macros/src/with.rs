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
            linker: linker.take().unwrap_or_else(|| quote!(emit::linker())),
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
            return Err(syn::Error::new(
                item.span(),
                "async functions aren't supported yet",
            ))
        }
        _ => return Err(syn::Error::new(item.span(), "unrecognized item type")),
    }

    Ok(quote!(#item))
}

fn inject_sync(props: &Props, linker_tokens: TokenStream, body: TokenStream) -> TokenStream {
    let props_tokens = props.props_tokens();

    quote!({
        let __link = {
            let mut __link = emit::ctxt::LinkGuard::new(#linker_tokens, #props_tokens);
            __link.activate();
            __link
        };

        #body
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_sync() {
        todo!()
    }

    #[test]
    fn inject_async() {
        todo!()
    }
}
