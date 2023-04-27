use proc_macro2::TokenStream;

use syn::{parse::Parse, spanned::Spanned, Attribute, FieldValue};

use crate::{
    args::{self, Arg},
    hook,
    util::FieldValueKey,
};

pub(super) struct Args {
    pub(super) inspect: bool,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut inspect = Arg::bool("inspect");

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut inspect],
        )?;

        Ok(Args {
            inspect: inspect.take_or_default(),
        })
    }
}

pub(super) fn key_value_with_hook(attrs: &[Attribute], fv: &FieldValue) -> TokenStream {
    let fn_name = match &*fv.key_name() {
        // Default to capturing the well-known error identifier as an error
        "err" => quote_spanned!(fv.span()=> __private_capture_as_error),
        // In other cases, capture using the default implementation
        _ => quote_spanned!(fv.span()=> __private_capture_as_default),
    };

    let key_expr = fv.key_expr();
    let expr = &fv.expr;

    quote_spanned!(fv.span()=>
        #(#attrs)*
        {
            use emit::__private::__PrivateCaptureHook;
            (emit::Key::new(#key_expr), (#expr).#fn_name())
        }
    )
}

pub(super) struct RenameHookTokens<T> {
    pub(super) args: TokenStream,
    pub(super) expr: TokenStream,
    pub(super) to: T,
}

pub(super) fn rename_hook_tokens(
    opts: RenameHookTokens<impl FnOnce(&Args) -> TokenStream>,
) -> Result<TokenStream, syn::Error> {
    hook::rename_hook_tokens(hook::RenameHookTokens {
        args: opts.args,
        expr: opts.expr,
        predicate: |ident: &str| ident.starts_with("__private_capture"),
        to: move |args: &Args| {
            let to_ident = (opts.to)(args);

            (to_ident, quote!())
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_value_with_hook_tokens() {
        let cases = vec![
            (
                quote!(
                    #[a]
                    #[b]
                    a
                ),
                quote!(
                    #[a]
                    #[b]
                    {
                        use emit::__private::__PrivateCaptureHook;
                        ("a", (a).__private_capture_as_default())
                    }
                ),
            ),
            (
                quote!(#[a] #[b] a: 42),
                quote!(
                    #[a]
                    #[b]
                    {
                        use emit::__private::__PrivateCaptureHook;
                        ("a", (42).__private_capture_as_default())
                    }
                ),
            ),
            (
                quote!(#[a] #[b] err: 42),
                quote!(
                    #[a]
                    #[b]
                    {
                        use emit::__private::__PrivateCaptureHook;
                        ("err", (42).__private_capture_as_error())
                    }
                ),
            ),
        ];

        for (expr, expected) in cases {
            let fv = syn::parse2::<FieldValue>(expr).unwrap();
            let attrs = &fv.attrs;

            let actual = key_value_with_hook(attrs, &fv);

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
