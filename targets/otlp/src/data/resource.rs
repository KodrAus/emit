use std::{collections::HashSet, ops::ControlFlow};

use sval_derive::Value;

use super::{AnyValue, EmitValue, KeyValue};

#[derive(Value)]
pub struct Resource<'a, A: ?Sized = InlineResourceAttributes<'a>> {
    #[sval(flatten)]
    pub attributes: &'a A,
    #[sval(index = 2)]
    pub dropped_attribute_count: u32,
}

const RESOURCE_ATTRIBUTES_LABEL: sval::Label =
    sval::Label::new("attributes").with_tag(&sval::tags::VALUE_IDENT);

const RESOURCE_ATTRIBUTES_INDEX: sval::Index = sval::Index::new(1);

#[derive(Value)]
pub struct InlineResourceAttributes<'a> {
    #[sval(index = 1)]
    pub attributes: &'a [KeyValue<&'a str, &'a AnyValue<'a>>],
}

pub struct EmitResourceAttributes<P>(pub P);

impl<P: emit_core::props::Props> sval::Value for EmitResourceAttributes<P> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.record_tuple_begin(None, None, None, None)?;

        stream.record_tuple_value_begin(
            None,
            &RESOURCE_ATTRIBUTES_LABEL,
            &RESOURCE_ATTRIBUTES_INDEX,
        )?;
        stream.seq_begin(None)?;

        let mut seen = HashSet::new();
        self.0.for_each(|k, v| {
            if seen.insert(k.to_owned()) {
                stream
                    .seq_value_begin()
                    .map(|_| ControlFlow::Continue(()))
                    .unwrap_or(ControlFlow::Break(()))?;

                sval_ref::stream_ref(
                    &mut *stream,
                    KeyValue {
                        key: k,
                        value: EmitValue(v),
                    },
                )
                .map(|_| ControlFlow::Continue(()))
                .unwrap_or(ControlFlow::Break(()))?;

                stream
                    .seq_value_end()
                    .map(|_| ControlFlow::Continue(()))
                    .unwrap_or(ControlFlow::Break(()))?;
            }

            ControlFlow::Continue(())
        });

        stream.seq_end()?;
        stream.record_tuple_value_end(
            None,
            &RESOURCE_ATTRIBUTES_LABEL,
            &RESOURCE_ATTRIBUTES_INDEX,
        )?;

        stream.record_tuple_end(None, None, None)
    }
}
