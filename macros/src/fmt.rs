use proc_macro2::TokenStream;
use syn::{parse::Parse, spanned::Spanned, Attribute, ExprLit, FieldValue};

use crate::{
    args::{self, Arg},
    hook,
};

pub(super) fn template_hole_with_hook(attrs: &[Attribute], hole: &ExprLit) -> TokenStream {
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

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut flags],
        )?;

        Ok(Args {
            flags: flags.take_or_default(),
        })
    }
}

impl Args {
    fn to_format_args(&self) -> String {
        if self.flags.is_empty() {
            "{}".to_owned()
        } else {
            // `:?b` -> `{:?b}`
            format!("{{:{}}}", self.flags)
        }
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
            let fmt = args.to_format_args();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_to_fmt() {
        for (args, expected) in [
            (Args { flags: "".to_owned() }, "{}"),
        ] {
            assert_eq!(expected, args.to_format_args());
        }
    }

    #[test]
    fn hook() {
        for (args, expr, expected) in [
            (quote!(), quote!(), quote!())
        ] {
            let actual = rename_hook_tokens(RenameHookTokens { args, expr }).unwrap();

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
