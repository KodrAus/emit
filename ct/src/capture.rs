/*!
Compile-time implementation of value capturing.

This module generates calls to `rt::capture`.
*/

use std::fmt::Write;

use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{
    parse::{self, Parse, ParseStream},
    punctuated::Punctuated,
    visit_mut::{self, VisitMut},
    Expr, ExprLit, ExprMacro, FieldValue, Lit, LitStr, Member,
};

pub(super) struct ExpandTokens<F: Fn(&str) -> TokenStream> {
    pub(super) expr: TokenStream,
    pub(super) fn_name: F,
}

pub(super) fn expand_tokens(opts: ExpandTokens<impl Fn(&str) -> TokenStream>) -> TokenStream {
    let key_value = syn::parse2::<FieldValue>(opts.expr).expect("failed to parse expr");

    let key_name = key_value.key_name().expect("expected a string literal");

    let fn_name = syn::parse2::<Ident>((opts.fn_name)(&key_name)).expect("failed to parse ident");

    expand(key_value, fn_name)
}

fn expand(key_value: FieldValue, fn_name: Ident) -> TokenStream {
    let key_expr = key_value.key_expr();
    let expr = key_value.expr;

    quote!(
        {
            extern crate emit;
            use emit::rt::__private::__PrivateCapture;
            (#key_expr, (#expr).#fn_name())
        }
    )
}

pub(super) struct RenameCaptureTokens<F: Fn(&str) -> bool, T: FnOnce(&Args) -> TokenStream> {
    pub(super) args: TokenStream,
    pub(super) expr: TokenStream,
    pub(super) predicate: F,
    pub(super) to: T,
}

struct RawArgs {
    fields: Punctuated<FieldValue, Token![,]>,
}

impl Parse for RawArgs {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        Ok(RawArgs {
            fields: input.parse_terminated(FieldValue::parse)?,
        })
    }
}

impl Args {
    fn from_raw(args: RawArgs) -> Self {
        let mut inspect = Default::default();

        // Don't accept any unrecognized field names
        for fv in args.fields {
            let name = fv.key_name();

            match name.as_deref() {
                Some("inspect") => {
                    inspect = match &fv.expr {
                        Expr::Lit(ExprLit {
                            lit: Lit::Bool(lit),
                            ..
                        }) => lit.value,
                        _ => panic!("the value of the `inspect` argument must be a literal `bool`"),
                    };
                }
                Some(unknown) => panic!("unexpected field `{}`", unknown),
                None => panic!("unexpected field <unnamed>"),
            }
        }

        Args { inspect }
    }
}

pub(super) struct Args {
    pub(super) inspect: bool,
}

pub(super) fn rename_capture_tokens(
    opts: RenameCaptureTokens<impl Fn(&str) -> bool, impl FnOnce(&Args) -> TokenStream>,
) -> TokenStream {
    let args = syn::parse2::<RawArgs>(opts.args).expect("failed to parse args");
    let expr = syn::parse2::<Expr>(opts.expr).expect("failed to parse expr");
    let to = syn::parse2::<Ident>((opts.to)(&Args::from_raw(args))).expect("failed to parse ident");

    if !matches!(expr, Expr::Macro(..)) {
        panic!("the emit attribute macros can only be placed on the outside of a field-value expression");
    }

    rename_capture(expr, opts.predicate, to)
}

fn rename_capture(mut expr: Expr, predicate: impl Fn(&str) -> bool, to: Ident) -> TokenStream {
    struct ReplaceLogDefaultMethod<F> {
        scratch: String,
        predicate: F,
        to: Ident,
    }

    impl<F> VisitMut for ReplaceLogDefaultMethod<F>
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

    ReplaceLogDefaultMethod {
        scratch: String::new(),
        predicate,
        to,
    }
    .visit_expr_mut(&mut expr);

    expr.to_token_stream()
}

pub(super) trait FieldValueExt {
    fn key_expr(&self) -> ExprLit;
    fn key_name(&self) -> Option<String> {
        match self.key_expr().lit {
            Lit::Str(s) => Some(s.value()),
            _ => None,
        }
    }
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
                    use emit::rt::__private::__PrivateCapture;
                    ("a", (a).__private_capture_as_default())
                }),
            ),
            (
                quote!(a: 42),
                quote!(__private_capture_as_default),
                quote!({
                    extern crate emit;
                    use emit::rt::__private::__PrivateCapture;
                    ("a", (42).__private_capture_as_default())
                }),
            ),
        ];

        for (expr, fn_name, expected) in cases {
            let actual = expand_tokens(ExpandTokens {
                expr,
                fn_name: |_| fn_name.clone(),
            });

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
            let actual = rename_capture_tokens(RenameCaptureTokens {
                args: quote!(),
                expr,
                predicate: |ident| ident.starts_with("__private"),
                to: |_| to,
            });

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
