/*!
Implementation details for `emit!` macros.

This crate is not intended to be consumed directly.
*/

extern crate proc_macro;

#[macro_use]
extern crate quote;

#[macro_use]
extern crate syn;

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
pub fn as_debug(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_capture_tokens(
        capture::RenameCaptureTokens {
            args: TokenStream::from(args),
            expr: TokenStream::from(item),
            predicate: |ident| ident.starts_with("__private_capture"),
            to: |args| {
                if args.capture {
                    quote!(__private_capture_as_debug)
                } else {
                    quote!(__private_capture_anon_as_debug)
                }
            },
        },
    ))
}

/**
Capture a key-value pair using its `Display` implementation.
*/
#[proc_macro_attribute]
pub fn as_display(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_capture_tokens(
        capture::RenameCaptureTokens {
            args: TokenStream::from(args),
            expr: TokenStream::from(item),
            predicate: |ident| ident.starts_with("__private_capture"),
            to: |args| {
                if args.capture {
                    quote!(__private_capture_as_display)
                } else {
                    quote!(__private_capture_anon_as_display)
                }
            },
        },
    ))
}

/**
Capture a key-value pair using its `sval::Value` implementation.
*/
#[proc_macro_attribute]
pub fn as_sval(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_capture_tokens(
        capture::RenameCaptureTokens {
            args: TokenStream::from(args),
            expr: TokenStream::from(item),
            predicate: |ident| ident.starts_with("__private_capture"),
            to: |args| {
                if args.capture {
                    quote!(__private_capture_as_sval)
                } else {
                    quote!(__private_capture_anon_as_sval)
                }
            },
        },
    ))
}

/**
Capture a key-value pair using its `serde::Serialize` implementation.
*/
#[proc_macro_attribute]
pub fn as_serde(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_capture_tokens(
        capture::RenameCaptureTokens {
            args: TokenStream::from(args),
            expr: TokenStream::from(item),
            predicate: |ident| ident.starts_with("__private_capture"),
            to: |args| {
                if args.capture {
                    quote!(__private_capture_as_serde)
                } else {
                    quote!(__private_capture_anon_as_serde)
                }
            },
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
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::rename_capture_tokens(
        capture::RenameCaptureTokens {
            args: TokenStream::from(args),
            expr: TokenStream::from(item),
            predicate: |ident| ident.starts_with("__private_capture"),
            to: |_| quote!(__private_capture_as_error),
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
            "source" => quote!(__private_capture_as_error),
            _ => quote!(__private_capture_as_default),
        },
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_as_debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_as_debug),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_anon_as_debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_anon_as_debug),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_as_display(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_as_display),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_anon_as_display(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_anon_as_display),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_as_sval(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_as_sval),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_anon_as_sval(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_anon_as_sval),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_as_serde(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_as_serde),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_anon_as_serde(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_anon_as_serde),
    }))
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_as_error(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |key| {
            if key != "source" {
                panic!("the #[source] attribute must use `source` as the key name")
            }

            quote!(__private_capture_as_error)
        },
    }))
}
