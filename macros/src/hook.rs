use std::fmt::Write;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::Parse,
    punctuated::Punctuated,
    token::Comma,
    visit_mut::{self, VisitMut},
    Expr, ExprMethodCall, Ident,
};

use crate::util::parse_comma_separated2;

pub struct RenameHookTokens<P, T> {
    pub args: TokenStream,
    pub expr: TokenStream,
    pub predicate: P,
    pub to: T,
}

pub fn rename_hook_tokens<A: Parse>(
    opts: RenameHookTokens<
        impl Fn(&str) -> bool,
        impl Fn(&A, &Ident, &Punctuated<Expr, Comma>) -> (TokenStream, TokenStream),
    >,
) -> Result<TokenStream, syn::Error> {
    let mut hook = syn::parse2::<Hook>(opts.expr)?;

    RenameVisitor {
        scratch: String::new(),
        predicate: opts.predicate,
        to: opts.to,
        args: syn::parse2::<A>(opts.args)?,
    }
    .visit_expr_mut(&mut hook.expr);

    Ok(hook.to_token_stream())
}

struct RenameVisitor<P, A, T> {
    scratch: String,
    predicate: P,
    args: A,
    to: T,
}

impl<P, A, T> VisitMut for RenameVisitor<P, A, T>
where
    P: Fn(&str) -> bool,
    T: Fn(&A, &Ident, &Punctuated<Expr, Comma>) -> (TokenStream, TokenStream),
{
    fn visit_expr_method_call_mut(&mut self, i: &mut ExprMethodCall) {
        let ExprMethodCall { method, args, .. } = i;

        self.scratch.clear();
        write!(&mut self.scratch, "{}", method).expect("infallible write to string");

        if (self.predicate)(&self.scratch) {
            let (to_ident_tokens, to_arg_tokens) = (self.to)(&self.args, &method, &args);

            *method = syn::parse2(to_ident_tokens).expect("invalid ident");
            *args = parse_comma_separated2(to_arg_tokens).expect("invalid args");
        }

        visit_mut::visit_expr_method_call_mut(self, i)
    }
}

/**
An expression with an optional trailing comma.

When reformatting the expression, the comma is discarded.
*/
struct Hook {
    expr: Expr,
}

impl Parse for Hook {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut items = input.parse_terminated(Expr::parse, Token![,])?;

        let expr = items
            .pop()
            .ok_or_else(|| syn::Error::new(input.span(), "missing expression"))?
            .into_value();

        if !items.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "expected a single expression",
            ));
        }

        Ok(Hook { expr })
    }
}

impl ToTokens for Hook {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Hook { expr } = self;

        tokens.extend(quote!(#expr));
    }
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
