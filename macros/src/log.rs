use proc_macro2::TokenStream;

pub(super) fn expand(_: TokenStream) -> TokenStream {
    /*
    Parse the input with 3 distinct kinds (in order):
    - top-level expressions, like `logger`
    - the message template, using `args`
    - the structured pairs, as fieldvalues between {}
    */
    quote! {
        panic!("lol")
    }
}
