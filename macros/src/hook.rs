use std::fmt::Write;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::Parse,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
    Expr, ExprMacro, Ident,
};

pub(super) struct RenameHookTokens<P, T> {
    pub(super) args: TokenStream,
    pub(super) expr: TokenStream,
    pub(super) predicate: P,
    pub(super) to: T,
}

pub(super) fn rename_hook_tokens<T: Parse>(
    opts: RenameHookTokens<impl Fn(&str) -> bool, impl FnOnce(&T) -> TokenStream>,
) -> Result<TokenStream, syn::Error> {
    let expr = syn::parse2::<Expr>(opts.expr)?;

    if !matches!(expr, Expr::Macro(..)) {
        return Err(syn::Error::new(opts.args.span(), "the emit attribute macros can only be placed on the outside of a field-value expression"));
    }

    let to = syn::parse2::<Ident>((opts.to)(&syn::parse2::<T>(opts.args)?))?;

    Ok(rename_capture(expr, opts.predicate, to))
}

fn rename_capture(mut expr: Expr, predicate: impl Fn(&str) -> bool, to: Ident) -> TokenStream {
    struct RenameVisitor<F> {
        scratch: String,
        predicate: F,
        to: Ident,
    }

    impl<F> VisitMut for RenameVisitor<F>
    where
        F: Fn(&str) -> bool,
    {
        fn visit_expr_mut(&mut self, node: &mut Expr) {
            if let Expr::Macro(ExprMacro { ref mut mac, .. }) = node {
                if let Some(last) = mac.path.segments.last_mut() {
                    self.scratch.clear();
                    write!(&mut self.scratch, "{}", last.ident)
                        .expect("infallible write to string");

                    if (self.predicate)(&self.scratch) {
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

    RenameVisitor {
        scratch: String::new(),
        predicate,
        to,
    }
    .visit_expr_mut(&mut expr);

    expr.to_token_stream()
}
