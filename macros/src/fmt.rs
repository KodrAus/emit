use proc_macro2::TokenStream;
use syn::{ExprLit, Attribute, spanned::Spanned, parse::Parse};

use crate::{hook, args::{Arg, self}};

pub(super) fn create_tokens(attrs: &[Attribute], hole: &ExprLit) -> TokenStream {
    quote_spanned!(hole.span()=>
        #(#attrs)*
        {
            extern crate emit;
            use emit::__private::__PrivateFmtHook;
            emit::template::Part::hole(#hole).__private_fmt_as_default()
        }
    )
}

pub(super) struct Args {
    pub(super) flags: String,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut flags = Arg::str("flags");

        args::set_from_parse2(input.cursor().token_stream(), [&mut flags])?;

        Ok(Args {
            flags: flags.take_or_default(),
        })
    }
}

pub(super) struct RenameHookTokens {
    pub(super) args: TokenStream,
    pub(super) expr: TokenStream,
}

pub(super) fn rename_hook_tokens(opts: RenameHookTokens) -> Result<TokenStream, syn::Error> {
    hook::rename_hook_tokens(hook::RenameHookTokens {
        args: opts.args,
        expr: opts.expr,
        predicate: |ident: &str| ident.starts_with("__private_fmt"),
        to: move |args: &Args| {
            let fmt = if args.flags.is_empty() {
                "{}".to_owned()
            } else {
                format!("{{:{}}}", args.flags)
            };
            
            let to_ident = quote!(__private_fmt_as);
            let to_arg = quote!(|v, f| {
                extern crate emit;
                use emit::__private::core::fmt;
                emit::__private::core::write!(f, #fmt, v)
            });

            (to_ident, to_arg)
        },
    })
}
