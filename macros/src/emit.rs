use proc_macro2::{Span, TokenStream};
use syn::{parse::Parse, FieldValue, Ident};

use crate::{
    args::{self, Arg},
    props, template,
};

pub struct ExpandTokens {
    pub receiver: TokenStream,
    pub level: TokenStream,
    pub input: TokenStream,
}

struct Args {
    extent: TokenStream,
    to: TokenStream,
    when: TokenStream,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut extent = Arg::token_stream("extent", |fv| {
            let expr = &fv.expr;

            Ok(quote!(#expr))
        });
        let mut to = Arg::token_stream("to", |fv| {
            let expr = &fv.expr;

            Ok(quote!(#expr))
        });
        let mut when = Arg::token_stream("when", |fv| {
            let expr = &fv.expr;

            Ok(quote!(#expr))
        });

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut extent, &mut to, &mut when],
        )?;

        Ok(Args {
            extent: extent.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
            to: to.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
            when: when.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let (args, template, mut props) = template::parse2::<Args>(opts.input)?;

    // Add the level as a property
    let level_ident = Ident::new(emit_core::well_known::LVL_KEY, Span::call_site());
    let level_value = opts.level;

    props.push(&syn::parse2::<FieldValue>(
        quote!(#level_ident: emit::Level::#level_value),
    )?)?;

    // Add the location as a property
    let loc_ident = Ident::new(emit_core::well_known::LOCATION_KEY, Span::call_site());
    props.push(&syn::parse2::<FieldValue>(
        quote!(#loc_ident: emit::__private::caller()),
    )?)?;

    let props_match_input_tokens = props.match_input_tokens();
    let props_match_binding_tokens = props.match_binding_tokens();
    let props_tokens = props.match_bound_tokens();

    let extent_tokens = args.extent;
    let to_tokens = args.to;
    let when_tokens = args.when;

    let template_tokens = template.template_tokens();

    let receiver_tokens = opts.receiver;

    Ok(quote!({
        match (#(#props_match_input_tokens),*) {
            (#(#props_match_binding_tokens),*) => {
                emit::#receiver_tokens(
                    #to_tokens,
                    #when_tokens,
                    #extent_tokens,
                    #template_tokens,
                    #props_tokens,
                )
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn expand_emit() {
        let cases = vec![
            (
                quote!("Text and {b: 17} and {a} and {#[as_debug] c} and {d: String::from(\"short lived\")} and {#[cfg(disabled)] e}"),
                quote!({
                    match (
                        {
                            use emit::__private::__PrivateCaptureHook;
                            (emit::Key::new("b"), (17).__private_capture_as_default())
                        },
                        {
                            use emit::__private::__PrivateCaptureHook;
                            (emit::Key::new("a"), (a).__private_capture_as_default())
                        },
                        #[as_debug]
                        {
                            use emit::__private::__PrivateCaptureHook;
                            (emit::Key::new("c"), (c).__private_capture_as_default())
                        },
                        {
                            use emit::__private::__PrivateCaptureHook;
                            (emit::Key::new("d"), (String::from ("short lived")).__private_capture_as_default())
                        },
                        #[cfg (disabled)]
                        {
                            {
                                use emit::__private::__PrivateCaptureHook;
                                (emit::Key::new("e"), (e).__private_capture_as_default())
                            }
                        },
                        #[cfg(not(disabled))] ()
                    ) {
                        (__tmp0, __tmp1, __tmp2, __tmp3, __tmp4) => {
                            emit::emit(
                                emit::target::Empty,
                                emit::filter::Empty,
                                emit::props::Empty,
                                emit::Level::Info,
                                None,
                                emit::Template::new_ref(&[
                                    emit::template::Part::text ("Text and "),
                                    {
                                        use emit::__private::__PrivateFmtHook;
                                        emit::template::Part::hole ("b").__private_fmt_as_default()
                                    },
                                    emit::template::Part::text (" and "),
                                    {
                                        use emit::__private::__PrivateFmtHook;
                                        emit::template::Part::hole ("a").__private_fmt_as_default()
                                    },
                                    emit::template::Part::text (" and "),
                                    #[as_debug]
                                    {
                                        use emit::__private::__PrivateFmtHook;
                                        emit::template::Part::hole ("c").__private_fmt_as_default()
                                    },
                                    emit::template::Part::text (" and "),
                                    {
                                        use emit::__private::__PrivateFmtHook;
                                        emit::template::Part::hole ("d").__private_fmt_as_default()
                                    },
                                    emit::template::Part::text (" and "),
                                    #[cfg (disabled)]
                                    {
                                        {
                                            use emit::__private::__PrivateFmtHook;
                                            emit::template::Part::hole ("e").__private_fmt_as_default()
                                        }
                                    }
                                ]),
                                emit::props::SortedSlice::new_ref(&[
                                    (__tmp1.0.by_ref(), __tmp1.1.by_ref()),
                                    (__tmp0.0.by_ref(), __tmp0.1.by_ref()),
                                    (__tmp2.0.by_ref(), __tmp2.1.by_ref()),
                                    (__tmp3.0.by_ref(), __tmp3.1.by_ref()),
                                    #[cfg(disabled)]
                                    (__tmp4.0.by_ref(), __tmp4.1.by_ref())
                                ]),
                            )
                        }
                    }
                }),
            ),
        ];

        for (expr, expected) in cases {
            let actual = expand_tokens(ExpandTokens {
                receiver: quote!(emit),
                level: quote!(Info),
                input: expr,
            }).unwrap();

            assert_eq!(expected.to_string(), actual.to_string());
        }
    }
}
