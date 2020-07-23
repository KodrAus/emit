use proc_macro2::TokenStream;
use syn::{Expr, ExprLit, Lit, LitStr, Meta, MetaNameValue};

pub(super) fn expand(meta: TokenStream, expr: TokenStream, fn_name: TokenStream) -> TokenStream {
    let meta: Option<Meta> = if !meta.is_empty() {
        Some(syn::parse2(meta).expect("failed to parse meta"))
    } else {
        None
    };
    let expr: Expr = syn::parse2(expr).expect("failed to parse expr");

    let mut key_expr = None;

    // Look for a `key = "str"` parameter first
    if let Some(Meta::NameValue(MetaNameValue { path, lit, .. })) = meta {
        if let Some(ident) = path.get_ident() {
            if ident == "key" {
                key_expr = Some(ExprLit { attrs: vec![], lit });
            }
        }
    }

    // If the key is empty, then try infer it from the value expression
    if key_expr.is_none() {
        if let Expr::Path(ref expr) = expr {
            if let Some(ident) = expr.path.get_ident() {
                key_expr = Some(ExprLit {
                    attrs: vec![],
                    lit: Lit::Str(LitStr::new(&ident.to_string(), ident.span())),
                });
            }
        }
    }

    let key_expr = key_expr.expect("could not determine key");

    quote! {
        {
            use antlog_macros_private::__private::__PrivateLogCapture;
            (#key_expr, (#expr).#fn_name())
        }
    }
}
