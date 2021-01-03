/*!
Implementation details for `emit!` macros.

This crate is not intended to be consumed directly.
*/

extern crate proc_macro;

#[macro_use]
extern crate quote;

use proc_macro2::TokenStream;

mod capture;
mod emit;
mod filter;

/**
Emit a trace record.
*/
#[proc_macro]
pub fn trace(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(item)
}

/**
Emit a debug record.
*/
#[proc_macro]
pub fn debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(item)
}

/**
Emit a info record.
*/
#[proc_macro]
pub fn info(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(item)
}

/**
Emit a warn record.
*/
#[proc_macro]
pub fn warn(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(item)
}

/**
Emit a error record.
*/
#[proc_macro]
pub fn error(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(item)
}

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
pub fn with_debug(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
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
pub fn with_display(
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
pub fn with_sval(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
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
pub fn with_serde(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
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

There should only be a single `#[source]` attribute per log statement.
It must use `source` as the key name.
*/
#[proc_macro_attribute]
pub fn source(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
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
            // A value with `source` as the key will be treated as the error by default
            "source" => quote!(__private_capture_from_error),
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
            if key != "source" {
                panic!("the #[source] attribute must use `source` as the key name")
            }

            quote!(__private_capture_from_error)
        },
    }))
}
