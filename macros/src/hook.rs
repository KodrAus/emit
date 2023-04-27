use std::fmt::Write;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::Parse,
    punctuated::Punctuated,
    visit_mut::{self, VisitMut},
    Expr, ExprMethodCall, Ident,
};

use crate::util::parse_comma_separated2;

pub(super) struct RenameHookTokens<P, T> {
    pub(super) args: TokenStream,
    pub(super) expr: TokenStream,
    pub(super) predicate: P,
    pub(super) to: T,
}

pub(super) fn rename_hook_tokens<T: Parse>(
    opts: RenameHookTokens<impl Fn(&str) -> bool, impl FnOnce(&T) -> (TokenStream, TokenStream)>,
) -> Result<TokenStream, syn::Error> {
    let mut expr = syn::parse2::<Expr>(opts.expr)?;

    let (to_ident_tokens, to_arg_tokens) = (opts.to)(&syn::parse2::<T>(opts.args)?);

    let to_ident = syn::parse2(to_ident_tokens)?;
    let to_args = parse_comma_separated2(to_arg_tokens)?;

    struct RenameVisitor<F> {
        scratch: String,
        predicate: F,
        to_ident: Ident,
        to_args: Punctuated<Expr, Token![,]>,
    }

    impl<F> VisitMut for RenameVisitor<F>
    where
        F: Fn(&str) -> bool,
    {
        fn visit_expr_mut(&mut self, node: &mut Expr) {
            if let Expr::MethodCall(ExprMethodCall { method, args, .. }) = node {
                self.scratch.clear();
                write!(&mut self.scratch, "{}", method).expect("infallible write to string");

                if (self.predicate)(&self.scratch) {
                    *method = self.to_ident.clone();
                    *args = self.to_args.clone();
                }
            }

            // Delegate to the default impl to visit nested expressions.
            visit_mut::visit_expr_mut(self, node);
        }
    }

    RenameVisitor {
        scratch: String::new(),
        predicate: opts.predicate,
        to_ident,
        to_args,
    }
    .visit_expr_mut(&mut expr);

    Ok(expr.to_token_stream())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_rename() {
        let cases = vec![
            (
                (
                    quote!(a.__private_capture()),
                    (quote!(__private_capture_as_debug), quote!()),
                ),
                quote!(a.__private_capture_as_debug()),
            ),
            (
                (
                    quote!(("a", 42.__private_capture())),
                    (quote!(__private_capture_as_debug), quote!(x, y, Z { z })),
                ),
                quote!(("a", 42.__private_capture_as_debug(x, y, Z { z }))),
            ),
        ];

        for ((expr, to), expected) in cases {
            let actual = rename_hook_tokens(RenameHookTokens {
                args: quote!({}),
                expr,
                predicate: |ident: &str| ident.starts_with("__private"),
                to: |_: &Expr| to,
            })
            .unwrap();

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
