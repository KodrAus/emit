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
Emit an error record.
*/
#[proc_macro]
pub fn error(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(quote!(ERROR), TokenStream::from(item))
}

/**
Format a template.
*/
#[proc_macro]
pub fn format(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit::expand_tokens(emit::ExpandTokens {
        receiver: quote!(__private_format),
        level: quote!(default()),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

fn emit(
    level: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> proc_macro::TokenStream {
    if filter::matches_build_filter() {
        emit::expand_tokens(emit::ExpandTokens {
            receiver: quote!(__private_emit),
            level,
            input: item,
        })
        .unwrap_or_compile_error()
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
    capture::rename_capture_tokens(capture::RenameCaptureTokens {
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
    })
    .unwrap_or_compile_error()
}

/**
Capture a key-value pair using its `Display` implementation.
*/
#[proc_macro_attribute]
pub fn as_display(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    capture::rename_capture_tokens(capture::RenameCaptureTokens {
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
    })
    .unwrap_or_compile_error()
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
        capture::rename_capture_tokens(capture::RenameCaptureTokens {
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
        })
        .unwrap_or_compile_error()
    }
    #[cfg(not(feature = "sval"))]
    {
        let _ = args;
        let _ = item;

        proc_macro::TokenStream::from(quote!(compile_error!(
            "capturing with `sval` is only possible when the `sval` Cargo feature is enabled"
        )))
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
        capture::rename_capture_tokens(capture::RenameCaptureTokens {
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
        })
        .unwrap_or_compile_error()
    }
    #[cfg(not(feature = "serde"))]
    {
        let _ = args;
        let _ = item;

        proc_macro::TokenStream::from(quote!(compile_error!(
            "capturing with `serde` is only possible when the `serde` Cargo feature is enabled"
        )))
    }
}

/**
Capture a key-value pair using its `Error` implementation.
*/
#[proc_macro_attribute]
pub fn as_error(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    #[cfg(feature = "std")]
    {
        capture::rename_capture_tokens(capture::RenameCaptureTokens {
            args: TokenStream::from(args),
            expr: TokenStream::from(item),
            predicate: |ident| ident.starts_with("__private_capture"),
            to: |_| quote!(__private_capture_as_error),
        })
        .unwrap_or_compile_error()
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = args;
        let _ = item;

        proc_macro::TokenStream::from(quote!(compile_error!(
            "capturing errors is only possible when the `std` Cargo feature is enabled"
        )))
    }
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |key| match key {
            // Default to capturing the well-known error identifier as an error
            emit_rt::well_known::ERR => quote!(__private_capture_as_error),
            // In other cases, capture using the default implementation
            _ => quote!(__private_capture_as_default),
        },
    })
    .unwrap_or_compile_error()
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_as_debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_as_debug),
    })
    .unwrap_or_compile_error()
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_anon_as_debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_anon_as_debug),
    })
    .unwrap_or_compile_error()
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_as_display(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_as_display),
    })
    .unwrap_or_compile_error()
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_anon_as_display(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_anon_as_display),
    })
    .unwrap_or_compile_error()
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_as_sval(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_as_sval),
    })
    .unwrap_or_compile_error()
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_anon_as_sval(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_anon_as_sval),
    })
    .unwrap_or_compile_error()
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_as_serde(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_as_serde),
    })
    .unwrap_or_compile_error()
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_anon_as_serde(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_anon_as_serde),
    })
    .unwrap_or_compile_error()
}

#[proc_macro]
#[doc(hidden)]
pub fn __private_capture_as_error(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    capture::expand_tokens(capture::ExpandTokens {
        expr: TokenStream::from(item),
        fn_name: |_| quote!(__private_capture_as_error),
    })
    .unwrap_or_compile_error()
}

trait TokenStreamExt {
    fn unwrap_or_compile_error(self) -> proc_macro::TokenStream;
}

impl TokenStreamExt for Result<TokenStream, syn::Error> {
    fn unwrap_or_compile_error(self) -> proc_macro::TokenStream {
        proc_macro::TokenStream::from(match self {
            Ok(item) => item,
            Err(err) => err.into_compile_error(),
        })
    }
}
