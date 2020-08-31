extern crate proc_macro;

#[macro_use]
extern crate quote;

use proc_macro2::TokenStream;

mod filter;
mod capture;
mod emit;

/**
Logging statements.
*/
#[proc_macro]
pub fn emit(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    if filter::matches_build_filter() {
        proc_macro::TokenStream::from(emit::expand_tokens(TokenStream::from(item)))
    } else {
        proc_macro::TokenStream::new()
    }
}

/**
Capture a key-value pair using its `Debug` implementation.
*/
#[proc_macro_attribute]
pub fn debug(_: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_capture_tokens(
        capture::RenameCaptureTokens {
            expr: TokenStream::from(item),
            predicate: |ident| ident.starts_with("__private_capture"),
            to: quote!(__private_capture_from_debug),
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
            predicate: |ident| ident.starts_with("__private_capture"),
            to: quote!(__private_capture_from_display),
        },
    ))
}

/**
Capture a key-value pair using its `sval::Value` implementation.
*/
#[proc_macro_attribute]
pub fn sval(_: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_capture_tokens(
        capture::RenameCaptureTokens {
            expr: TokenStream::from(item),
            predicate: |ident| ident.starts_with("__private_capture"),
            to: quote!(__private_capture_from_sval),
        },
    ))
}

/**
Capture a key-value pair using its `serde::Serialize` implementation.
*/
#[proc_macro_attribute]
pub fn serde(_: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_capture_tokens(
        capture::RenameCaptureTokens {
            expr: TokenStream::from(item),
            predicate: |ident| ident.starts_with("__private_capture"),
            to: quote!(__private_capture_from_serde),
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
            predicate: |ident| ident.starts_with("__private_capture"),
            to: quote!(__private_capture_from_error),
        },
    ))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |key| match key {
            "err" => quote!(__private_capture_from_error),
            _ => quote!(__private_capture_with_default),
        },
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_from_debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_from_debug),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_from_display(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_from_display),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_from_sval(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_from_sval),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_from_serde(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_from_serde),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_from_error(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |key| {
            if key != "err" {
                panic!("the #[error] attribute must use `err` as the key name")
            }

            quote!(__private_capture_from_error)
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
