use proc_macro2::TokenStream;
use syn::{spanned::Spanned, Expr, ExprLit, FieldValue, Lit};

use crate::util::{print_list, FieldValueKey};

/**
An argument represented as a field-value input to a macro.

Arguments are set from a collection of field-values using either the `set_from_parse2` or `set_from_field_values` functions.
*/
pub struct Arg<T> {
    key: &'static str,
    set: Box<dyn FnMut(&FieldValue) -> Result<T, syn::Error>>,
    value: Option<T>,
}

impl Arg<bool> {
    pub fn bool(key: &'static str) -> Self {
        Arg::new(key, move |fv| {
            if let Expr::Lit(ExprLit {
                lit: Lit::Bool(ref l),
                ..
            }) = fv.expr
            {
                Ok(l.value)
            } else {
                Err(syn::Error::new(
                    fv.expr.span(),
                    format_args!("`{}` requires a boolean value", key),
                ))
            }
        })
    }
}

impl Arg<String> {
    pub fn str(key: &'static str) -> Self {
        Arg::new(key, move |fv| {
            if let Expr::Lit(ExprLit {
                lit: Lit::Str(ref l),
                ..
            }) = fv.expr
            {
                Ok(l.value())
            } else {
                Err(syn::Error::new(
                    fv.expr.span(),
                    format_args!("`{}` requires a string value", key),
                ))
            }
        })
    }
}

impl Arg<TokenStream> {
    pub fn token_stream(
        key: &'static str,
        to_tokens: impl FnMut(&FieldValue) -> Result<TokenStream, syn::Error> + 'static,
    ) -> Self {
        Arg::new(key, to_tokens)
    }
}

impl<T> Arg<T> {
    pub fn new(
        key: &'static str,
        to_custom: impl FnMut(&FieldValue) -> Result<T, syn::Error> + 'static,
    ) -> Self {
        Arg {
            key,
            set: Box::new(to_custom),
            value: None,
        }
    }

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
    fn set(&mut self, fv: &FieldValue) -> Result<(), syn::Error>;
}

impl<T> ArgDef for Arg<T> {
    fn key(&self) -> &str {
        self.key
    }

    fn set(&mut self, fv: &FieldValue) -> Result<(), syn::Error> {
        if self.value.is_some() {
            return Err(syn::Error::new(
                fv.span(),
                format_args!("a value for `{}` has already been specified", self.key),
            ));
        }

        self.value = Some((self.set)(fv)?);
        Ok(())
    }
}

pub fn set_from_field_values<'a, const N: usize>(
    field_values: impl Iterator<Item = &'a FieldValue> + 'a,
    mut args: [&mut dyn ArgDef; N],
) -> Result<(), syn::Error> {
    'fields: for fv in field_values {
        let key_name = fv.key_name();

        for arg in &mut args {
            if arg.key() == key_name {
                arg.set(fv)?;
                continue 'fields;
            }
        }

        return Err(syn::Error::new(
            fv.span(),
            format_args!(
                "unknown argument `{}`; available arguments are {}",
                key_name,
                print_list(|| args.iter().map(|arg| arg.key()))
            ),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::util::parse_comma_separated2;

    use super::*;

    #[test]
    fn arg_set() {
        let mut bool = Arg::bool("bool");
        let mut str = Arg::str("str");
        let mut ts = Arg::token_stream("ts", |e| Ok(quote!(#e)));

        let fv =
            parse_comma_separated2::<FieldValue>(quote!(bool: true, ts: |a| { b }, str: "str"))
                .unwrap();

        set_from_field_values(fv.iter(), [&mut bool, &mut str, &mut ts]).unwrap();

        assert_eq!(true, bool.take().unwrap());
        assert_eq!("str", str.take().unwrap());
        assert_eq!(
            quote!(|a| { b }).to_string(),
            ts.take().unwrap().to_string()
        );
    }

    #[test]
    fn arg_err() {
        for (expected, fv) in [
            (
                "a value for `a` has already been specified",
                quote!(a: true, a: false),
            ),
            ("unknown argument `b`", quote!(b: true)),
            ("`a` requires a boolean value", quote!(a: "true")),
        ] {
            let mut a = Arg::bool("a");

            let fv = parse_comma_separated2::<FieldValue>(fv).unwrap();

            let err = set_from_field_values(fv.iter(), [&mut a]).unwrap_err();

            assert_eq!(expected, err.to_string());
        }
    }
}
