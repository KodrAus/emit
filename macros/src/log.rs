use std::mem;

use antlog_template::ct::{Part, Template};
use proc_macro2::TokenStream;
use syn::LitStr;

pub(super) fn expand_tokens(lit: TokenStream) -> TokenStream {
    /*
    Parse the input with 3 distinct kinds (in order):
    - top-level expressions, like `logger`
    - the message template, using `args`
    - the structured pairs, as fieldvalues between {}
    */

    let lit = syn::parse2::<LitStr>(lit).expect("failed to parse lit");
    let lit = lit.value();

    let template = Template::parse(&lit).expect("failed to parse");
    let fields = template.parts.into_iter().filter_map(|part| {
        if let Part::Hole { mut expr, .. } = part {
            let attrs = mem::replace(&mut expr.attrs, vec![]);

            Some(quote!(#(#attrs)* antlog_macros::__log_private_capture!(#expr)))
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
