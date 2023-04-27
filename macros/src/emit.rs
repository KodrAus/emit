use proc_macro2::TokenStream;
use syn::{parse::Parse, FieldValue};

use crate::{
    args::{self, Arg},
    template,
};

pub(super) struct ExpandTokens {
    pub(super) receiver: TokenStream,
    pub(super) level: TokenStream,
    pub(super) input: TokenStream,
}

struct Args {
    to: TokenStream,
    when: TokenStream,
    with: TokenStream,
    ts: TokenStream,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut to = Arg::token_stream("to", |expr| Ok(quote!(Some(#expr))));
        let mut when = Arg::token_stream("when", |expr| Ok(quote!(Some(#expr))));
        let mut with = Arg::token_stream("with", |expr| Ok(quote!(Some(#expr))));
        let mut ts = Arg::token_stream("ts", |expr| Ok(quote!(Some(#expr))));

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut to, &mut when, &mut with, &mut ts],
        )?;

        Ok(Args {
            to: to.take().unwrap_or_else(|| quote!(emit::target::Discard)),
            when: when.take().unwrap_or_else(|| quote!(emit::filter::Always)),
            with: with.take().unwrap_or_else(|| quote!(emit::ctxt::Empty)),
            ts: ts.take().unwrap_or_else(|| quote!(None)),
        })
    }
}

pub(super) fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let (args, template, props) = template::parse2::<Args>(opts.input)?;

    let props_match_value_tokens = props.match_value_tokens();
    let props_match_binding_tokens = props.match_binding_tokens();
    let props_tokens = props.sorted_key_value_tokens();

    let to_tokens = args.to;
    let when_tokens = args.when;
    let with_tokens = args.with;
    let ts_tokens = args.ts;

    let level_tokens = {
        let level = opts.level;
        quote!(emit::Level::#level)
    };

    let template_tokens = template.template_tokens();

    let receiver_tokens = opts.receiver;

    Ok(quote!({
        extern crate emit;

        match (#(#props_match_value_tokens),*) {
            (#(#props_match_binding_tokens),*) => {
                emit::#receiver_tokens(
                    #to_tokens,
                    #when_tokens,
                    #with_tokens,
                    #level_tokens,
                    #ts_tokens,
                    #template_tokens,
                    &[#(#props_tokens),*],
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
                    extern crate emit;
                    match (
                        {
                            extern crate emit;
                            use emit::__private::__PrivateCaptureHook;
                            ("b", (17).__private_capture_as_default())
                        },
                        {
                            extern crate emit;
                            use emit::__private::__PrivateCaptureHook;
                            ("a", (a).__private_capture_as_default())
                        },
                        #[as_debug]
                        {
                            extern crate emit;
                            use emit::__private::__PrivateCaptureHook;
                            ("c", (c).__private_capture_as_default())
                        },
                        {
                            extern crate emit;
                            use emit::__private::__PrivateCaptureHook;
                            ("d", (String::from ("short lived")).__private_capture_as_default())
                        },
                        #[cfg (disabled)]
                        {
                            {
                                extern crate emit;
                                use emit::__private::__PrivateCaptureHook;
                                ("e", (e).__private_capture_as_default())
                            }
                        },
                        #[cfg(not(disabled))] ()
                    ) {
                        (__tmp0, __tmp1, __tmp2, __tmp3, __tmp4) => {
                            emit::emit(
                                emit::target::Discard,
                                emit::filter::Always,
                                emit::ctxt::Empty,
                                emit::Level::Info, None,
                                emit::Template::new_ref(&[
                                    emit::template::Part::text ("Text and "),
                                    {
                                        extern crate emit;
                                        use emit::__private::__PrivateFmtHook;
                                        emit::template::Part::hole ("b").__private_fmt_as_default()
                                    },
                                    emit::template::Part::text (" and "),
                                    {
                                        extern crate emit;
                                        use emit::__private::__PrivateFmtHook;
                                        emit::template::Part::hole ("a").__private_fmt_as_default()
                                    },
                                    emit::template::Part::text (" and "),
                                    #[as_debug]
                                    {
                                        extern crate emit;
                                        use emit::__private::__PrivateFmtHook;
                                        emit::template::Part::hole ("c").__private_fmt_as_default()
                                    },
                                    emit::template::Part::text (" and "),
                                    {
                                        extern crate emit;
                                        use emit::__private::__PrivateFmtHook;
                                        emit::template::Part::hole ("d").__private_fmt_as_default()
                                    },
                                    emit::template::Part::text (" and "),
                                    #[cfg (disabled)]
                                    {
                                        {
                                            extern crate emit;
                                            use emit::__private::__PrivateFmtHook;
                                            emit::template::Part::hole ("e").__private_fmt_as_default()
                                        }
                                    }
                                ]),
                                &[
                                    (emit::Key::new (__tmp1.0), __tmp1.1.by_ref()),
                                    (emit::Key::new (__tmp0.0), __tmp0.1.by_ref()),
                                    (emit::Key::new (__tmp2.0), __tmp2.1.by_ref()),
                                    (emit::Key::new (__tmp3.0), __tmp3.1.by_ref()),
                                    #[cfg(disabled)]
                                    (emit::Key::new (__tmp4.0), __tmp4.1.by_ref())
                                ],
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
