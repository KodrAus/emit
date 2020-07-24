use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{Expr, ExprMethodCall, FieldValue, Member, Lit, ExprLit, LitStr, visit_mut::{self, VisitMut}};

const DEFAULT_METHOD: &'static str = "__private_log_capture_with_default";

pub(super) fn expand_default(expr: TokenStream) -> TokenStream {
    let key_value = syn::parse2::<FieldValue>(expr).expect("failed to parse expr");

    let key_expr = ExprLit {
        attrs: vec![],
        lit: Lit::Str(match key_value.member {
            Member::Named(member) => LitStr::new(&member.to_string(), member.span()),
            Member::Unnamed(member) => LitStr::new(&member.index.to_string(), member.span),
        }),
    };

    let expr = key_value.expr;
    let method = Ident::new(DEFAULT_METHOD, Span::call_site());

    quote!(
        {
            use antlog_macros_private::__private::__PrivateLogCapture;
            (#key_expr, (#expr).#method())
        }
    )
}

pub(super) fn expand(expr: TokenStream, fn_name: TokenStream) -> TokenStream {
    struct ReplaceLogDefaultMethod {
        with: Ident,
    }
    
    impl VisitMut for ReplaceLogDefaultMethod {
        fn visit_expr_mut(&mut self, node: &mut Expr) {
            if let Expr::MethodCall(ExprMethodCall { ref mut method, .. }) = node {
                if method == DEFAULT_METHOD {
                    *method = self.with.clone()
                }
            }
    
            // Delegate to the default impl to visit nested expressions.
            visit_mut::visit_expr_mut(self, node);
        }
    }

    let mut expr = syn::parse2::<Expr>(expr)
        .expect("failed to parse expr");

    let fn_name = syn::parse2::<Ident>(fn_name).expect("failed to parse ident");

    ReplaceLogDefaultMethod { with: fn_name }.visit_expr_mut(&mut expr);

    expr.to_token_stream()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_capture_default() {
        let cases = vec![
            (
                quote!(a),
                quote!({
                    use antlog_macros_private::__private::__PrivateLogCapture;
                    ("a", (a).__private_log_capture_with_default())
                }),
            ),
            (
                quote!(a: 42),
                quote!({
                    use antlog_macros_private::__private::__PrivateLogCapture;
                    ("a", (42).__private_log_capture_with_default())
                }),
            ),
        ];

        for (expr, expected) in cases {
            let actual = expand_default(expr);

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }

    #[test]
    fn expand_capture() {
        let cases = vec![
            (
                (
                    expand_default(quote!(a)),
                    quote!(__private_log_capture_from_debug),
                ),
                quote!({
                    use antlog_macros_private::__private::__PrivateLogCapture;
                    ("a", (a).__private_log_capture_from_debug())
                }),
            ),
            (
                (
                    expand_default(quote!(a: 42)),
                    quote!(__private_log_capture_from_display),
                ),
                quote!({
                    use antlog_macros_private::__private::__PrivateLogCapture;
                    ("a", (42).__private_log_capture_from_display())
                }),
            ),
        ];

        for ((expr, fn_name), expected) in cases {
            let actual = expand(expr, fn_name);

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
