use proc_macro2::TokenStream;
use syn::{
    spanned::Spanned,
    Expr, ExprLit, FieldValue, Lit,
};

use crate::util::{FieldValueKey, parse_comma_separated2};

/**
An argument represented as a field-value input to a macro.

Arguments are set from a collection of field-values using either the `set_from_parse2` or `set_from_field_values` functions.
*/
pub struct Arg<T> {
    key: &'static str,
    set: Box<dyn FnMut(&Expr) -> Result<T, syn::Error>>,
    value: Option<T>,
}

impl Arg<bool> {
    pub fn bool(key: &'static str) -> Self {
        Arg {
            key,
            set: Box::new(move |expr| {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Bool(l), ..
                }) = expr
                {
                    Ok(l.value)
                } else {
                    Err(syn::Error::new(
                        expr.span(),
                        format_args!("{} requires a boolean value", key),
                    ))
                }
            }),
            value: None,
        }
    }
}

impl Arg<String> {
    pub fn str(key: &'static str) -> Self {
        Arg {
            key,
            set: Box::new(move |expr| {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(l), ..
                }) = expr
                {
                    Ok(l.value())
                } else {
                    Err(syn::Error::new(
                        expr.span(),
                        format_args!("{} requires a string value", key),
                    ))
                }
            }),
            value: None,
        }
    }
}

impl Arg<TokenStream> {
    pub fn token_stream(
        key: &'static str,
        to_tokens: impl FnMut(&Expr) -> Result<TokenStream, syn::Error> + 'static,
    ) -> Self {
        Arg {
            key,
            set: Box::new(to_tokens),
            value: None,
        }
    }
}

impl<T> Arg<T> {
    pub fn take(self) -> Option<T> {
        self.value
    }
}

impl<T: Default> Arg<T> {
    pub fn take_or_default(self) -> T {
        self.take().unwrap_or_default()
    }
}

pub trait ArgDef {
    fn key(&self) -> &str;
    fn set(&mut self, expr: &Expr) -> Result<(), syn::Error>;
}

impl<T> ArgDef for Arg<T> {
    fn key(&self) -> &str {
        self.key
    }

    fn set(&mut self, expr: &Expr) -> Result<(), syn::Error> {
        if self.value.is_some() {
            return Err(syn::Error::new(
                expr.span(),
                format_args!("{} has already been specified", self.key),
            ));
        }

        self.value = Some((self.set)(expr)?);
        Ok(())
    }
}

pub fn set_from_parse2<const N: usize>(
    tokens: TokenStream,
    args: [&mut dyn ArgDef; N],
) -> Result<(), syn::Error> {
    set_from_field_values(parse_comma_separated2::<FieldValue>(tokens)?.iter(), args)
}

pub fn set_from_field_values<'a, const N: usize>(
    field_values: impl Iterator<Item = &'a FieldValue> + 'a,
    args: [&mut dyn ArgDef; N],
) -> Result<(), syn::Error> {
    for fv in field_values {
        let key_name = fv.key_name();

        for arg in args {
            if arg.key() == key_name {
                arg.set(&fv.expr)?;
                continue;
            }
        }

        return Err(syn::Error::new(
            fv.span(),
            format_args!("unexpected field `{}`", key_name),
        ));
    }

    Ok(())
}
