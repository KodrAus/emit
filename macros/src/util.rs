use proc_macro2::TokenStream;
use syn::{
    parse::{self, Parse, ParseStream},
    punctuated::Punctuated,
    Attribute, ExprLit, FieldValue, Lit, LitStr, MacroDelimiter, Member, Meta, MetaList,
};

pub trait FieldValueKey {
    fn key_expr(&self) -> ExprLit;

    fn key_name(&self) -> String {
        match self.key_expr().lit {
            Lit::Str(s) => s.value(),
            _ => panic!("invalid key expression"),
        }
    }
}

impl FieldValueKey for FieldValue {
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

pub trait AttributeCfg {
    fn is_cfg(&self) -> bool;
    fn invert_cfg(&self) -> Option<Attribute>;
}

impl AttributeCfg for Attribute {
    fn is_cfg(&self) -> bool {
        if let Some(ident) = self.path().get_ident() {
            ident == "cfg"
        } else {
            false
        }
    }

    fn invert_cfg(&self) -> Option<Attribute> {
        match self.path().get_ident() {
            Some(ident) if ident == "cfg" => {
                let tokens = match &self.meta {
                    Meta::Path(meta) => quote!(not(#meta)),
                    Meta::List(meta) => {
                        let meta = &meta.tokens;
                        quote!(not(#meta))
                    }
                    Meta::NameValue(meta) => quote!(not(#meta)),
                };

                Some(Attribute {
                    pound_token: self.pound_token.clone(),
                    style: self.style.clone(),
                    bracket_token: self.bracket_token.clone(),
                    meta: Meta::List(MetaList {
                        path: self.path().clone(),
                        delimiter: MacroDelimiter::Paren(Default::default()),
                        tokens,
                    }),
                })
            }
            _ => None,
        }
    }
}

pub fn parse_comma_separated2<T: Parse>(
    tokens: TokenStream,
) -> Result<Punctuated<T, Token![,]>, syn::Error> {
    struct ParsePunctuated<T> {
        value: Punctuated<T, Token![,]>,
    }

    impl<T: Parse> Parse for ParsePunctuated<T> {
        fn parse(input: ParseStream) -> parse::Result<Self> {
            Ok(ParsePunctuated {
                value: input.parse_terminated(T::parse, Token![,])?,
            })
        }
    }

    Ok(syn::parse2::<ParsePunctuated<T>>(tokens)?.value)
}
