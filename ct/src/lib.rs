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
Emit a debug record.
*/
#[proc_macro]
pub fn debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(quote!(DEBUG), TokenStream::from(item))
}

/**
Emit a info record.
*/
#[proc_macro]
pub fn info(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(quote!(INFO), TokenStream::from(item))
}

/**
Emit a warn record.
*/
#[proc_macro]
pub fn warn(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(quote!(WARN), TokenStream::from(item))
}

/**
Emit a error record.
*/
#[proc_macro]
pub fn error(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(quote!(ERROR), TokenStream::from(item))
}

fn emit(
    level: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> proc_macro::TokenStream {
    if filter::matches_build_filter() {
        proc_macro::TokenStream::from(emit::expand_tokens(level, item, quote!(__private_emit)))
    } else {
        proc_macro::TokenStream::new()
    }
}

/**
Format a template.
*/
#[proc_macro]
pub fn format(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(emit::expand_tokens(
        TokenStream::from(item),
        quote!(__private_format),
        quote!(default()),
    ))
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
                if args.inspect {
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
                if args.inspect {
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
    #[cfg(feature = "sval")]
    {
        proc_macro::TokenStream::from(capture::rename_capture_tokens(
            capture::RenameCaptureTokens {
                args: TokenStream::from(args),
                expr: TokenStream::from(item),
                predicate: |ident| ident.starts_with("__private_capture"),
                to: |args| {
                    if args.inspect {
                        quote!(__private_capture_as_sval)
                    } else {
                        quote!(__private_capture_anon_as_sval)
                    }
                },
            },
        ))
    }
    #[cfg(not(feature = "sval"))]
    {
        let _ = args;
        let _ = item;

        panic!("capturing with `sval` is only possible when the `sval` Cargo feature is enabled")
    }
}

/**
Capture a key-value pair using its `serde::Serialize` implementation.
*/
#[proc_macro_attribute]
pub fn as_serde(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    #[cfg(feature = "serde")]
    {
        proc_macro::TokenStream::from(capture::rename_capture_tokens(
            capture::RenameCaptureTokens {
                args: TokenStream::from(args),
                expr: TokenStream::from(item),
                predicate: |ident| ident.starts_with("__private_capture"),
                to: |args| {
                    if args.inspect {
                        quote!(__private_capture_as_serde)
                    } else {
                        quote!(__private_capture_anon_as_serde)
                    }
                },
            },
        ))
    }
    #[cfg(not(feature = "serde"))]
    {
        let _ = args;
        let _ = item;

        panic!("capturing with `serde` is only possible when the `serde` Cargo feature is enabled")
    }
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
    #[cfg(feature = "std")]
    {
        proc_macro::TokenStream::from(capture::rename_capture_tokens(
            capture::RenameCaptureTokens {
                args: TokenStream::from(args),
                expr: TokenStream::from(item),
                predicate: |ident| ident.starts_with("__private_capture"),
                to: |_| quote!(__private_capture_as_error),
            },
        ))
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = args;
        let _ = item;

        panic!("capturing errors is only possible when the `std` Cargo feature is enabled")
    }
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_as_default),
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
