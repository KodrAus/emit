use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{
    visit_mut::{self, VisitMut},
    Expr, ExprLit, ExprMacro, FieldValue, Lit, LitStr, Member,
};

pub(super) trait FieldValueExt {
    fn key_expr(&self) -> ExprLit;
}

impl FieldValueExt for FieldValue {
    fn key_expr(&self) -> ExprLit {
        ExprLit {
            attrs: vec![],
            lit: Lit::Str(match self.member {
                Member::Named(ref member) => LitStr::new(&member.to_string(), member.span()),
                Member::Unnamed(ref member) => LitStr::new(&member.index.to_string(), member.span),
            }),
        }
    }
}

pub(super) fn expand(key_value: FieldValue, fn_name: Ident) -> TokenStream {
    let key_expr = key_value.key_expr();
    let expr = key_value.expr;

    quote!(
        {
            use antlog_macros_impl::__private::__PrivateLogCapture;
            (#key_expr, (#expr).#fn_name())
        }
    )
}

pub(super) fn expand_tokens(expr: TokenStream, fn_name: TokenStream) -> TokenStream {
    let key_value = syn::parse2::<FieldValue>(expr).expect("failed to parse expr");
    let fn_name = syn::parse2::<Ident>(fn_name).expect("failed to parse ident");

    expand(key_value, fn_name)
}

pub(super) fn rename_default(mut expr: Expr, from: Ident, to: Ident) -> TokenStream {
    struct ReplaceLogDefaultMethod {
        from: Ident,
        to: Ident,
    }

    impl VisitMut for ReplaceLogDefaultMethod {
        fn visit_expr_mut(&mut self, node: &mut Expr) {
            if let Expr::Macro(ExprMacro { ref mut mac, .. }) = node {
                if let Some(last) = mac.path.segments.last_mut() {
                    if last.ident == self.from {
                        let span = last.ident.span();

                        // Set the name of the identifier, retaining its original span
                        last.ident = self.to.clone();
                        last.ident.set_span(span);
                    }
                }
            }

            // Delegate to the default impl to visit nested expressions.
            visit_mut::visit_expr_mut(self, node);
        }
    }

    ReplaceLogDefaultMethod { from, to }.visit_expr_mut(&mut expr);

    expr.to_token_stream()
}

pub(super) fn rename_default_tokens(expr: TokenStream, from: TokenStream, to: TokenStream) -> TokenStream {
    let expr = syn::parse2::<Expr>(expr).expect("failed to parse expr");
    let from = syn::parse2::<Ident>(from).expect("failed to parse ident");
    let to = syn::parse2::<Ident>(to).expect("failed to parse ident");

    rename_default(expr, from, to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_capture() {
        let cases = vec![
            (
                quote!(a),
                quote!(__private_log_capture_with_default),
                quote!({
                    use antlog_macros_impl::__private::__PrivateLogCapture;
                    ("a", (a).__private_log_capture_with_default())
                }),
            ),
            (
                quote!(a: 42),
                quote!(__private_log_capture_with_default),
                quote!({
                    use antlog_macros_impl::__private::__PrivateLogCapture;
                    ("a", (42).__private_log_capture_with_default())
                }),
            ),
        ];

        for (expr, fn_name, expected) in cases {
            let actual = expand_tokens(expr, fn_name);

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }

    #[test]
    fn expand_rename() {
        let cases = vec![
            (
                (
                    quote!(__log_private_capture!(a)),
                    quote!(__log_private_capture),
                    quote!(__log_private_capture_debug),
                ),
                quote!(__log_private_capture_debug!(a)),
            ),
            (
                (
                    quote!(log::__log_private_capture!(a: 42)),
                    quote!(__log_private_capture),
                    quote!(__log_private_capture_debug),
                ),
                quote!(log::__log_private_capture_debug!(a: 42)),
            ),
        ];

        for ((expr, from, to), expected) in cases {
            let actual = rename_default_tokens(expr, from, to);

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
