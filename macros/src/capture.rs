use proc_macro2::TokenStream;

use syn::{
    parse::Parse, punctuated::Punctuated, spanned::Spanned, token::Comma, Attribute, Expr,
    FieldValue, Ident,
};

use crate::{
    args::{self, Arg},
    hook, key,
    util::FieldValueKey,
};

pub struct Args {
    pub inspect: bool,
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

pub fn key_value_with_hook(
    attrs: &[Attribute],
    fv: &FieldValue,
    interpolated: bool,
    captured: bool,
) -> TokenStream {
    let fn_name = match &*fv.key_name() {
        emit_core::well_known::LVL_KEY => quote_spanned!(fv.span()=> __private_capture_as_level),
        emit_core::well_known::ERR_KEY => quote_spanned!(fv.span()=> __private_capture_as_error),
        emit_core::well_known::SPAN_ID_KEY => {
            quote_spanned!(fv.span()=> __private_capture_as_span_id)
        }
        emit_core::well_known::SPAN_PARENT_KEY => {
            quote_spanned!(fv.span()=> __private_capture_as_span_id)
        }
        emit_core::well_known::TRACE_ID_KEY => {
            quote_spanned!(fv.span()=> __private_capture_as_trace_id)
        }
        // In other cases, capture using the default implementation
        _ => quote_spanned!(fv.span()=> __private_capture_as_default),
    };

    let key_expr = fv.key_expr();
    let expr = &fv.expr;

    let interpolated_expr = if interpolated {
        quote!(.__private_interpolated())
    } else {
        quote!(.__private_uninterpolated())
    };

    let captured_expr = if captured {
        quote!(.__private_captured())
    } else {
        quote!(.__private_uncaptured())
    };

    let key_tokens = key::key_with_hook(&[], &key_expr);
    let value_tokens = quote_spanned!(fv.span()=> #[allow(unused_imports)] {
        use emit::__private::{__PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _, __PrivateOptionalMapHook as _, __PrivateInterpolatedHook as _};
        (#expr).__private_optional_capture_some().__private_optional_map_some(|v| v.#fn_name()) #interpolated_expr #captured_expr
    });

    quote_spanned!(fv.span()=>
        #(#attrs)*
        {
            (#key_tokens, #value_tokens)
        }
    )
}

pub struct RenameHookTokens<T> {
    pub name: &'static str,
    pub args: TokenStream,
    pub expr: TokenStream,
    pub to: T,
}

pub fn rename_hook_tokens(
    opts: RenameHookTokens<impl Fn(&Args) -> TokenStream>,
) -> Result<TokenStream, syn::Error> {
    hook::rename_hook_tokens(hook::RenameHookTokens {
        name: opts.name,
        target: "values in `emit` macros",
        args: opts.args,
        expr: opts.expr,
        predicate: |ident: &str| {
            ident.starts_with("__private_capture") || ident.starts_with("__private_captured")
        },
        to: move |hook_args: &Args, ident: &Ident, args: &Punctuated<Expr, Comma>| {
            if ident.to_string().starts_with("__private_captured") {
                return None;
            }

            let to_ident = (opts.to)(hook_args);

            Some((to_ident, quote!(#args)))
        },
    })
}
