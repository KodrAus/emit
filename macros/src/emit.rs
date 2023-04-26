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

        match (#((#props_match_value_tokens)),*) {
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
                        {emit::__private_capture!(b: 17) },
                        {emit::__private_capture!(a) },
                        {
                            #[as_debug]
                            emit::__private_capture!(c)
                        },
                        {emit::__private_capture!(d: String::from("short lived")) },
                        #[cfg(disabled)]
                        {emit::__private_capture!(e) },
                        #[cfg(not(disabled))]
                        ()
                    ) {
                        (__tmp0, __tmp1, __tmp2, __tmp3, __tmp4) => {
                            let event = emit::RawEvent {
                                ts: emit::Timestamp::now(),
                                lvl: emit::Level::Info,
                                props: &[
                                    (__tmp1.0, __tmp1.1.by_ref()),
                                    (__tmp0.0, __tmp0.1.by_ref()),
                                    (__tmp2.0, __tmp2.1.by_ref()),
                                    (__tmp3.0, __tmp3.1.by_ref()),
                                    #[cfg(disabled)]
                                    (__tmp4.0, __tmp4.1.by_ref())
                                ],
                                tpl: emit::template::Template::new(&[
                                    emit::template::Part::Text("Text and "),
                                    emit::template::Part::Hole ( "b"),
                                    emit::template::Part::Text(" and "),
                                    emit::template::Part::Hole ( "a"),
                                    emit::template::Part::Text(" and "),
                                    emit::template::Part::Hole ( "c" ),
                                    emit::template::Part::Text(" and "),
                                    emit::template::Part::Hole ( "d" ),
                                    emit::template::Part::Text(" and "),
                                    #[cfg(disabled)]
                                    emit::template::Part::Hole ( "e" )
                                ]),
                            };

                            emit::__private_emit!({
                                to: None,
                                key_value_cfgs: [
                                    #[cfg(not(emit_rt__private_false))],
                                    #[cfg(not(emit_rt__private_false))],
                                    #[cfg(not(emit_rt__private_false))],
                                    #[cfg(not(emit_rt__private_false))],
                                    #[cfg(disabled)]
                                ],
                                keys: ["a", "b", "c", "d", #[cfg(disabled)] "e"],
                                values: [&__tmp1, &__tmp0, &__tmp2, &__tmp3, #[cfg(disabled)] &__tmp4],
                                event: &event,
                            })
                        }
                    }
                }),
            ),
            (
                quote!(to: log, "Text and {a}", a: 42),
                quote!({
                    extern crate emit;

                    match (
                        { emit::__private_capture!(a: 42) }
                    ) {
                        (__tmp0) => {
                            let event = emit::Event {
                                ts: emit::Timestamp::now(),
                                lvl: emit::Level::Info,
                                props: &[(__tmp0.0, __tmp0.1.by_ref())],
                                tpl: emit::template::Template::new(&[
                                    emit::template::Part::Text("Text and "),
                                    emit::template::Part::Hole ( "a")
                                ]),
                            };

                            emit::__private_emit!({
                                to: Some(log),
                                key_value_cfgs: [
                                    #[cfg(not(emit_rt__private_false))]
                                ],
                                keys: ["a"],
                                values: [&__tmp0],
                                event: &event,
                            })
                        }
                    }
                })
            )
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
