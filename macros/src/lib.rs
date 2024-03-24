/*!
Implementation details for `emit!` macros.

This crate is not intended to be consumed directly.
*/

extern crate proc_macro;

#[macro_use]
extern crate quote;

#[macro_use]
extern crate syn;

use std::collections::HashMap;

use proc_macro2::TokenStream;

mod args;
mod build;
mod capture;
mod emit;
mod event;
mod filter;
mod fmt;
mod format;
mod hook;
mod key;
mod module;
mod optional;
mod props;
mod span;
mod template;
mod util;

use util::ResultToTokens;

fn hooks() -> HashMap<&'static str, fn(TokenStream, TokenStream) -> syn::Result<TokenStream>> {
    let mut map = HashMap::new();

    map.insert(
        "fmt",
        (|args: TokenStream, expr: TokenStream| {
            fmt::rename_hook_tokens(fmt::RenameHookTokens { args, expr })
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map.insert(
        "key",
        (|args: TokenStream, expr: TokenStream| {
            key::rename_hook_tokens(key::RenameHookTokens { args, expr })
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map.insert(
        "optional",
        (|args: TokenStream, expr: TokenStream| {
            optional::rename_hook_tokens(optional::RenameHookTokens { args, expr })
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map.insert(
        "as_value",
        (|args: TokenStream, expr: TokenStream| {
            capture_as(
                "as_value",
                args,
                expr,
                quote!(__private_capture_as_value),
                quote!(__private_capture_anon_as_value),
            )
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map.insert(
        "as_debug",
        (|args: TokenStream, expr: TokenStream| {
            capture_as(
                "as_debug",
                args,
                expr,
                quote!(__private_capture_as_debug),
                quote!(__private_capture_anon_as_debug),
            )
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map.insert(
        "as_display",
        (|args: TokenStream, expr: TokenStream| {
            capture_as(
                "as_display",
                args,
                expr,
                quote!(__private_capture_as_display),
                quote!(__private_capture_anon_as_display),
            )
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map.insert(
            "as_sval",
            (|args: TokenStream, expr: TokenStream| {
                #[cfg(feature = "sval")]
                {
                    capture_as(
                        "as_sval",
                        args,
                        expr,
                        quote!(__private_capture_as_sval),
                        quote!(__private_capture_anon_as_sval),
                    )
                }
                #[cfg(not(feature = "sval"))]
                {
                    let _ = args;

                    Err(syn::Error::new(expr.span(), "capturing with `sval` is only possible when the `sval` Cargo feature is enabled"))
                }
            }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>
        );

    map.insert(
            "as_serde",
            (|args: TokenStream, expr: TokenStream| {
                #[cfg(feature = "serde")]
                {
                    capture_as(
                        "as_serde",
                        args,
                        expr,
                        quote!(__private_capture_as_serde),
                        quote!(__private_capture_anon_as_serde),
                    )
                }
                #[cfg(not(feature = "serde"))]
                {
                    let _ = args;

                    Err(syn::Error::new(expr.span(), "capturing with `serde` is only possible when the `serde` Cargo feature is enabled"))
                }
            }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>
        );

    map.insert(
        "as_error",
        (|args: TokenStream, expr: TokenStream| {
            #[cfg(feature = "std")]
            {
                capture_as(
                    "as_error",
                    args,
                    expr,
                    quote!(__private_capture_as_error),
                    quote!(__private_capture_as_error),
                )
            }
            #[cfg(not(feature = "std"))]
            {
                let _ = args;

                Err(syn::Error::new(
                    expr.span(),
                    "capturing errors is only possible when the `std` Cargo feature is enabled",
                ))
            }
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map
}

/**
Format a template.
*/
#[proc_macro]
pub fn format(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    format::expand_tokens(format::ExpandTokens {
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Construct an event.
*/
#[proc_macro]
pub fn event(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    event::expand_tokens(event::ExpandTokens {
        level: None,
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

/**
Construct an event.
*/
#[proc_macro]
pub fn debug_event(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    event::expand_tokens(event::ExpandTokens {
        level: Some(quote!(Debug)),
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

/**
Construct an event.
*/
#[proc_macro]
pub fn info_event(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    event::expand_tokens(event::ExpandTokens {
        level: Some(quote!(Info)),
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

/**
Construct an event.
*/
#[proc_macro]
pub fn warn_event(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    event::expand_tokens(event::ExpandTokens {
        level: Some(quote!(Warn)),
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

/**
Construct an event.
*/
#[proc_macro]
pub fn error_event(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    event::expand_tokens(event::ExpandTokens {
        level: Some(quote!(Error)),
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

/**
Wrap an operation in a span.
*/
#[proc_macro_attribute]
pub fn span(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    base_span(None, TokenStream::from(args), TokenStream::from(item))
}

/**
Wrap an operation in a span.
*/
#[proc_macro_attribute]
pub fn debug_span(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    base_span(
        Some(quote!(Debug)),
        TokenStream::from(args),
        TokenStream::from(item),
    )
}

/**
Wrap an operation in a span.
*/
#[proc_macro_attribute]
pub fn info_span(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    base_span(
        Some(quote!(Info)),
        TokenStream::from(args),
        TokenStream::from(item),
    )
}

/**
Wrap an operation in a span.
*/
#[proc_macro_attribute]
pub fn warn_span(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    base_span(
        Some(quote!(Warn)),
        TokenStream::from(args),
        TokenStream::from(item),
    )
}

/**
Wrap an operation in a span.
*/
#[proc_macro_attribute]
pub fn error_span(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    base_span(
        Some(quote!(Error)),
        TokenStream::from(args),
        TokenStream::from(item),
    )
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

/**
Get the parts of a template.
*/
#[proc_macro]
pub fn tpl_parts(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_template_parts_tokens(build::ExpandTemplateTokens {
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Emit an event.
*/
#[proc_macro]
pub fn emit(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    base_emit(None, TokenStream::from(item))
}

/**
Emit a debug event.
*/
#[proc_macro]
pub fn debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    base_emit(Some(quote!(Debug)), TokenStream::from(item))
}

/**
Emit a info event.
*/
#[proc_macro]
pub fn info(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    base_emit(Some(quote!(Info)), TokenStream::from(item))
}

/**
Emit a warn event.
*/
#[proc_macro]
pub fn warn(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    base_emit(Some(quote!(Warn)), TokenStream::from(item))
}

/**
Emit an error event.
*/
#[proc_macro]
pub fn error(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    base_emit(Some(quote!(Error)), TokenStream::from(item))
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

#[proc_macro_attribute]
pub fn fmt(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("fmt").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

#[proc_macro_attribute]
pub fn key(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("key").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

#[proc_macro_attribute]
pub fn optional(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("optional").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

/**
Capture a key-value pair using its `ToValue` implementation.
*/
#[proc_macro_attribute]
pub fn as_value(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("as_value").unwrap())(TokenStream::from(args), TokenStream::from(item))
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
    (hook::get("as_debug").unwrap())(TokenStream::from(args), TokenStream::from(item))
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
    (hook::get("as_display").unwrap())(TokenStream::from(args), TokenStream::from(item))
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
    (hook::get("as_sval").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

/**
Capture a key-value pair using its `serde::Serialize` implementation.
*/
#[proc_macro_attribute]
pub fn as_serde(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("as_serde").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

/**
Capture a key-value pair using its `Error` implementation.
*/
#[proc_macro_attribute]
pub fn as_error(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("as_error").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

fn base_emit(level: Option<TokenStream>, item: TokenStream) -> proc_macro::TokenStream {
    if filter::matches_build_filter() {
        emit::expand_tokens(emit::ExpandTokens { level, input: item }).unwrap_or_compile_error()
    } else {
        proc_macro::TokenStream::new()
    }
}

fn base_span(
    level: Option<TokenStream>,
    input: TokenStream,
    item: TokenStream,
) -> proc_macro::TokenStream {
    if filter::matches_build_filter() {
        span::expand_tokens(span::ExpandTokens { level, input, item }).unwrap_or_compile_error()
    } else {
        item.into()
    }
}

fn capture_as(
    name: &'static str,
    args: TokenStream,
    expr: TokenStream,
    as_fn: TokenStream,
    as_anon_fn: TokenStream,
) -> syn::Result<TokenStream> {
    capture::rename_hook_tokens(capture::RenameHookTokens {
        name,
        args,
        expr,
        to: |args: &capture::Args| {
            if args.inspect {
                as_fn.clone()
            } else {
                as_anon_fn.clone()
            }
        },
    })
}
