use std::mem;

use antlog_template::ct::{Part, Template};
use proc_macro2::{Span, TokenStream, TokenTree};
use syn::LitStr;

pub(super) fn expand_tokens(lit: TokenStream) -> TokenStream {
    /*
    Parse the input with 3 distinct kinds (in order):
    - top-level expressions, like `logger`
    - the message template, using `args`
    - the structured pairs, as fieldvalues between {}
    */

    let src = match lit.clone().into_iter().next() {
        Some(TokenTree::Literal(src)) => src,
        _ => panic!("expected a string literal"),
    };

    let lit = syn::parse2::<LitStr>(lit).expect("failed to parse lit");
    let lit = lit.value();

    let template = Template::parse(&lit).expect("failed to parse");
    let fields = template.parts.into_iter().filter_map(|part| {
        if let Part::Hole { mut expr, range } = part {
            let attrs = mem::replace(&mut expr.attrs, vec![]);

            // The range needs to be offset by 1 to account for the leading `"`
            // TODO: Consider raw strings too
            let field_span = src.subspan(range.start + 1 .. range.end + 1).unwrap_or_else(Span::call_site);

            // TODO: Handle `String::from` case, we need to use `match`:
            // match ((a).capture(), (b).capture(), (c).capture()) {
            //     (a, b, c) => { .. }
            // }
            Some(quote_spanned!(field_span=> #(#attrs)* antlog_macros::__log_private_capture!(#expr)))
        } else {
            None
        }
    });

    // TODO: Make this return a `private::Captured`
    quote!(&[
        #(#fields),*
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_log() {
        let cases = vec![(
            quote!("Text and {a} and {b: 17} and {#[debug] c}"),
            quote!(&[
                antlog_macros::__log_private_capture!(a),
                antlog_macros::__log_private_capture!(b: 17),
                #[debug]
                antlog_macros::__log_private_capture!(c)
            ]),
        )];

        for (expr, expected) in cases {
            let actual = expand_tokens(expr);

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
