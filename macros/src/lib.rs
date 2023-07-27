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

mod args;
mod build;
mod capture;
mod emit;
mod filter;
mod fmt;
mod hook;
mod key;
mod props;
mod span;
mod template;
mod util;
mod with;

use util::ResultToTokens;

/**
Emit a debug record.
*/
#[proc_macro]
pub fn debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(quote!(Debug), TokenStream::from(item))
}

/**
Emit a info record.
*/
#[proc_macro]
pub fn info(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(quote!(Info), TokenStream::from(item))
}

/**
Emit a warn record.
*/
#[proc_macro]
pub fn warn(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(quote!(Warn), TokenStream::from(item))
}

/**
Emit an error record.
*/
#[proc_macro]
pub fn error(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit(quote!(Error), TokenStream::from(item))
}

/**
Format a template.
*/
#[proc_macro]
pub fn format(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit::expand_tokens(emit::ExpandTokens {
        receiver: quote!(__private::__format),
        level: quote!(default()),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Construct a set of properties.
*/
#[proc_macro]
pub fn props(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_props_tokens(build::ExpandPropsTokens {
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Construct a template.
*/
#[proc_macro]
pub fn tpl(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_template_tokens(build::ExpandTemplateTokens {
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[proc_macro_attribute]
pub fn fmt(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    fmt::rename_hook_tokens(fmt::RenameHookTokens {
        args: TokenStream::from(args),
        expr: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[proc_macro_attribute]
pub fn with(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    with::expand_tokens(with::ExpandTokens {
        sync_receiver: quote!(__private::__with),
        async_receiver: quote!(__private::__with_future),
        input: TokenStream::from(args),
        item: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[proc_macro_attribute]
pub fn key(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    key::rename_hook_tokens(key::RenameHookTokens {
        args: TokenStream::from(args),
        expr: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Capture a key-value pair using its `Debug` implementation.
*/
#[proc_macro_attribute]
pub fn as_debug(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    capture_as(
        TokenStream::from(args),
        TokenStream::from(item),
        quote!(__private_capture_as_debug),
        quote!(__private_capture_anon_as_debug),
    )
}

/**
Capture a key-value pair using its `Display` implementation.
*/
#[proc_macro_attribute]
pub fn as_display(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    capture_as(
        TokenStream::from(args),
        TokenStream::from(item),
        quote!(__private_capture_as_display),
        quote!(__private_capture_anon_as_display),
    )
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
        capture_as(
            TokenStream::from(args),
            TokenStream::from(item),
            quote!(__private_capture_as_sval),
            quote!(__private_capture_anon_as_sval),
        )
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
        capture_as(
            TokenStream::from(args),
            TokenStream::from(item),
            quote!(__private_capture_as_serde),
            quote!(__private_capture_anon_as_serde),
        )
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
        capture_as(
            TokenStream::from(args),
            TokenStream::from(item),
            quote!(__private_capture_as_error),
            quote!(__private_capture_as_error),
        )
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

fn emit(level: TokenStream, item: TokenStream) -> proc_macro::TokenStream {
    if filter::matches_build_filter() {
        emit::expand_tokens(emit::ExpandTokens {
            receiver: quote!(__private::__emit),
            level,
            input: item,
        })
        .unwrap_or_compile_error()
    } else {
        proc_macro::TokenStream::new()
    }
}

fn capture_as(
    args: TokenStream,
    expr: TokenStream,
    as_fn: TokenStream,
    as_anon_fn: TokenStream,
) -> proc_macro::TokenStream {
    capture::rename_hook_tokens(capture::RenameHookTokens {
        args: TokenStream::from(args),
        expr: TokenStream::from(expr),
        to: |args: &capture::Args| {
            if args.inspect {
                as_fn
            } else {
                as_anon_fn
            }
        },
    })
    .unwrap_or_compile_error()
}
