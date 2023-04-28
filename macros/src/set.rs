use proc_macro2::TokenStream;
use syn::{parse::Parse, spanned::Spanned, Block, FieldValue, Item, ItemFn};

use crate::{
    args::{self, Arg},
    template,
};

pub struct ExpandTokens {
    pub item: TokenStream,
    pub input: TokenStream,
}

struct Args {
    to: TokenStream,
    when: TokenStream,
    linker: TokenStream,
    ts: TokenStream,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut to = Arg::token_stream("to", |expr| Ok(quote!(#expr)));
        let mut when = Arg::token_stream("when", |expr| Ok(quote!(#expr)));
        let mut linker = Arg::token_stream("linker", |expr| Ok(quote!(#expr)));
        let mut ts = Arg::token_stream("ts", |expr| Ok(quote!(Some(#expr))));

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut to, &mut when, &mut linker, &mut ts],
        )?;

        Ok(Args {
            to: to.take().unwrap_or_else(|| quote!(emit::target::Discard)),
            when: when.take().unwrap_or_else(|| quote!(emit::filter::Always)),
            linker: linker.take().unwrap_or_else(|| quote!(emit::linker())),
            ts: ts.take().unwrap_or_else(|| quote!(None)),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let (args, _, props) = template::parse2::<Args>(opts.input)?;

    let linker_tokens = args.linker;

    // TODO: Make this a visitor
    let mut item = syn::parse2::<Item>(opts.item)?;
    match &mut item {
        Item::Fn(ItemFn { block, .. }) => {
            let props_token = props.props_tokens();

            **block = syn::parse2::<Block>(quote!({
                let __link = {
                    let mut __link = emit::__private::__PrivateLink::new(#linker_tokens, #props_token);
                    __link.activate();
                    __link
                };

                #block
            }))?;
        }
        _ => return Err(syn::Error::new(item.span(), "unrecognized item type")),
    }

    Ok(quote!(#item))
}
