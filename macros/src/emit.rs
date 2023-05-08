use proc_macro2::TokenStream;
use syn::{parse::Parse, FieldValue};

use crate::{
    args::{self, Arg},
    template,
};

pub struct ExpandTokens {
    pub receiver: TokenStream,
    pub level: TokenStream,
    pub input: TokenStream,
}

struct Args {
    to: TokenStream,
    when: TokenStream,
    with: TokenStream,
    ts: TokenStream,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut to = Arg::token_stream("to", |expr| Ok(quote!(#expr)));
        let mut when = Arg::token_stream("when", |expr| Ok(quote!(#expr)));
        let mut with = Arg::token_stream("with", |expr| Ok(quote!(#expr)));
        let mut ts = Arg::token_stream("ts", |expr| Ok(quote!(Some(#expr))));

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut to, &mut when, &mut with, &mut ts],
        )?;

        Ok(Args {
            to: to.take().unwrap_or_else(|| quote!(emit::target::Empty)),
            when: when.take().unwrap_or_else(|| quote!(emit::filter::Empty)),
            with: with.take().unwrap_or_else(|| quote!(emit::props::Empty)),
            ts: ts.take().unwrap_or_else(|| quote!(None)),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let (args, template, props) = template::parse2::<Args>(opts.input)?;

    let props_match_input_tokens = props.match_input_tokens();
    let props_match_binding_tokens = props.match_binding_tokens();
    let props_tokens = props.match_bound_tokens();

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
        match (#(#props_match_input_tokens),*) {
            (#(#props_match_binding_tokens),*) => {
                emit::#receiver_tokens(
                    #to_tokens,
                    #when_tokens,
                    #with_tokens,
                    #level_tokens,
                    #ts_tokens,
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
                                emit::target::Discard,
                                emit::filter::Always,
                                emit::props::Empty,
                                emit::Level::Info, None,
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
