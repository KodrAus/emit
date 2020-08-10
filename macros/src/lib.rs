extern crate proc_macro;

#[macro_use]
extern crate quote;

use proc_macro2::TokenStream;

mod capture;
mod log;
mod template;

/**
Logging statements.
*/
#[proc_macro]
pub fn log(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(log::expand_tokens(TokenStream::from(item)))
}

/**
Capture a key-value pair using its `Debug` implementation.
*/
#[proc_macro_attribute]
pub fn debug(_: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_default_tokens(
        TokenStream::from(item),
        quote!(__log_private_capture),
        quote!(__log_private_capture_debug),
    ))
}

/**
Capture a key-value pair using its `Display` implementation.
*/
#[proc_macro_attribute]
pub fn display(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_default_tokens(
        TokenStream::from(item),
        quote!(__log_private_capture),
        quote!(__log_private_capture_display),
    ))
}

/**
Capture an Error.

There should only be a single `#[error]` attribute per log statement.
It will always use `"error"` as the key.
*/
#[proc_macro_attribute]
pub fn error(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_default_tokens(
        TokenStream::from(item),
        quote!(__log_private_capture),
        quote!(__log_private_capture_error),
    ))
}

// TODO: Also add `error` (which sets the key name to `error` too)

#[proc_macro]
#[doc(hidden)]
pub fn __log_private_capture(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(
        TokenStream::from(item),
        quote!(__private_log_capture_with_default),
        None,
    ))
}

#[proc_macro]
#[doc(hidden)]
pub fn __log_private_capture_debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(
        TokenStream::from(item),
        quote!(__private_log_capture_from_debug),
        None,
    ))
}

#[proc_macro]
#[doc(hidden)]
pub fn __log_private_capture_display(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(
        TokenStream::from(item),
        quote!(__private_log_capture_from_display),
        None,
    ))
}

#[proc_macro]
#[doc(hidden)]
pub fn __log_private_capture_error(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(
        TokenStream::from(item),
        quote!(__private_log_capture_from_error),
        Some(quote!("error")),
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn ui() {
        let t = trybuild::TestCases::new();
        t.pass("tests/ui/pass/*.rs");
        t.compile_fail("tests/ui/fail/*.rs");
    }
}
