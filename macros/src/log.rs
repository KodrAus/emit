use std::mem;

use antlog_template::ct::{Part, Template};
use proc_macro2::{Span, TokenStream, TokenTree};
use syn::{Ident, LitStr};

use crate::capture::FieldValueExt;

pub(super) fn expand_tokens(lit: TokenStream) -> TokenStream {
    let src = match lit.clone().into_iter().next() {
        Some(TokenTree::Literal(src)) => src,
        _ => panic!("expected a string literal"),
    };

    let lit = syn::parse2::<LitStr>(lit).expect("failed to parse lit");
    let lit = lit.value();

    let template = Template::parse(&lit).expect("failed to parse");

    // Extract the field values from the template
    // Each field value goes into a match-arm binding (to support short-lived values like `String::new()`)
    // Each field value also gets a match-arm that converts its key string into an index lookup
    let mut field_values = Vec::new();
    let mut field_bindings = Vec::new();
    let mut field_lookups = Vec::new();

    let mut field_index = 0usize;
    for part in template.parts.into_iter() {
        if let Part::Hole { mut expr, range } = part {
            let attrs = mem::replace(&mut expr.attrs, vec![]);
            let field_span = src
                .subspan(range.start + 1..range.end + 1)
                .unwrap_or_else(Span::call_site);
            let key_expr = expr.key_expr();

            field_values.push(quote_spanned!(field_span=> #(#attrs)* antlog_macros::__log_private_capture!(#expr)));
            field_bindings.push(Ident::new(
                &format!("__tmp{}", field_index),
                Span::call_site(),
            ));
            field_lookups.push(quote!(#key_expr => Some(#field_index)));

            field_index += 1;
        }
    }

    field_lookups.push(quote!(_ => None));

    quote!({
        match (#(#field_values),*) {
            (#(#field_bindings),*) => {
                let captured = antlog_macros_impl::__private::Captured {
                    lookup: |k| match k { #(#field_lookups),* },
                    key_values: &[#(#field_bindings),*]
                };

                println!("{:?}", captured.key_values);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn expand_log() {
        let cases = vec![(
            quote!("Text and {a} and {b: 17} and {#[debug] c}"),
            quote!({
                match (
                    antlog_macros::__log_private_capture!(a),
                    antlog_macros::__log_private_capture!(b: 17),
                    #[debug]
                    antlog_macros::__log_private_capture!(c)
                ) {
                    (__tmp0, __tmp1, __tmp2) => {
                        let captured = antlog_macros_impl::__private::Captured {
                            lookup: |k| match k {
                                "a" => Some(0usize),
                                "b" => Some(1usize),
                                "c" => Some(2usize),
                                _ => None
                            },
                            key_values: &[__tmp0, __tmp1, __tmp2]
                        };
                        println!("{:?}", captured.key_values);
                    }
                }
            }),
        )];

        for (expr, expected) in cases {
            let actual = expand_tokens(expr);

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
