/*!
Compile-time implementation of value capturing.

This module generates calls to `rt::capture`.
*/

use proc_macro2::{Ident, TokenStream};

use syn::{parse::Parse, FieldValue};

use crate::{
    args::{self, Arg},
    util::FieldValueKey,
};

pub(super) struct ExpandTokens<F: Fn(&str) -> TokenStream> {
    pub(super) expr: TokenStream,
    pub(super) fn_trait: TokenStream,
    pub(super) fn_name: F,
}

pub(super) fn expand_tokens(
    opts: ExpandTokens<impl Fn(&str) -> TokenStream>,
) -> Result<TokenStream, syn::Error> {
    let key_value = syn::parse2::<FieldValue>(opts.expr)?;

    let key_name = key_value.key_name();

    let fn_name = syn::parse2::<Ident>((opts.fn_name)(&key_name))?;

    Ok(expand(key_value, opts.fn_trait, fn_name))
}

fn expand(key_value: FieldValue, fn_trait: TokenStream, fn_name: Ident) -> TokenStream {
    let key_expr = key_value.key_expr();
    let expr = key_value.expr;

    quote!(
        {
            extern crate emit;
            use emit::__private::#fn_trait;
            (#key_expr, (#expr).#fn_name())
        }
    )
}

pub(super) struct Args {
    pub(super) inspect: bool,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut inspect = Arg::bool("inspect");

        args::set_from_parse2(input.cursor().token_stream(), [&mut inspect])?;

        Ok(Args {
            inspect: inspect.take_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_capture() {
        let cases = vec![
            (
                quote!(a),
                quote!(__private_capture_as_default),
                quote!({
                    extern crate emit;
                    use emit::__private::__PrivateCaptureHook;
                    ("a", (a).__private_capture_as_default())
                }),
            ),
            (
                quote!(a: 42),
                quote!(__private_capture_as_default),
                quote!({
                    extern crate emit;
                    use emit::__private::__PrivateCaptureHook;
                    ("a", (42).__private_capture_as_default())
                }),
            ),
        ];

        for (expr, fn_name, expected) in cases {
            let actual = expand_tokens(ExpandTokens {
                expr,
                fn_name: |_| fn_name.clone(),
            })
            .unwrap();

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }

    #[test]
    fn expand_rename() {
        let cases = vec![
            (
                (
                    quote!(__private_capture!(a)),
                    quote!(__private_capture_as_debug),
                ),
                quote!(__private_capture_as_debug!(a)),
            ),
            (
                (
                    quote!(log::__private_capture!(a: 42)),
                    quote!(__private_capture_as_debug),
                ),
                quote!(log::__private_capture_as_debug!(a: 42)),
            ),
        ];

        for ((expr, to), expected) in cases {
            let actual = rename_hook_tokens(RenameHookTokens {
                args: quote!(),
                expr,
                predicate: |ident| ident.starts_with("__private"),
                to: |_| to,
            })
            .unwrap();

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
