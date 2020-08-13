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
    proc_macro::TokenStream::from(log::rearrange_tokens(TokenStream::from(item)))
}

/**
Capture a key-value pair using its `Debug` implementation.
*/
#[proc_macro_attribute]
pub fn debug(_: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_capture_tokens(
        capture::RenameCaptureTokens {
            expr: TokenStream::from(item),
            predicate: |ident| ident.starts_with("__private_log_capture"),
            to: quote!(__private_log_capture_from_debug),
        },
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
    proc_macro::TokenStream::from(capture::rename_capture_tokens(
        capture::RenameCaptureTokens {
            expr: TokenStream::from(item),
            predicate: |ident| ident.starts_with("__private_log_capture"),
            to: quote!(__private_log_capture_from_display),
        },
    ))
}

/**
Capture a key-value pair using its `sval::Value` implementation.
*/
#[proc_macro_attribute]
pub fn sval(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_capture_tokens(
        capture::RenameCaptureTokens {
            expr: TokenStream::from(item),
            predicate: |ident| ident.starts_with("__private_log_capture"),
            to: quote!(__private_log_capture_from_sval),
        },
    ))
}

/**
Capture an Error.

There should only be a single `#[error]` attribute per log statement.
It must use `err` as the key name.
*/
#[proc_macro_attribute]
pub fn error(_: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_capture_tokens(
        capture::RenameCaptureTokens {
            expr: TokenStream::from(item),
            predicate: |ident| ident.starts_with("__private_log_capture"),
            to: quote!(__private_log_capture_from_error),
        },
    ))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_log(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(log::expand_tokens(TokenStream::from(item)))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_log_capture(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |key| match key {
            "err" => quote!(__private_log_capture_from_error),
            _ => quote!(__private_log_capture_with_default),
        },
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_log_capture_from_debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_log_capture_from_debug),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_log_capture_from_display(
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_log_capture_from_display),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_log_capture_from_sval(
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_log_capture_from_sval),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_log_capture_from_error(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |key| {
            if key != "err" {
                panic!("the #[error] attribute must use `err` as the key name")
            }

            quote!(__private_log_capture_from_error)
        },
    }))
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
