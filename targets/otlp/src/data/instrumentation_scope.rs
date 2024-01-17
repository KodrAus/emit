use sval_derive::Value;

use super::{stream_attributes, stream_field, AnyValue, KeyValue};

#[derive(Value)]
pub struct InstrumentationScope<'a, A: ?Sized = InlineInstrumentationScopeAttributes<'a>> {
    #[sval(label = "name", index = 1)]
    pub name: &'a str,
    #[sval(label = "version", index = 2)]
    pub version: &'a str,
    #[sval(flatten)]
    pub attributes: &'a A,
    #[sval(label = "droppedAttributeCount", index = 4)]
    pub dropped_attribute_count: u32,
}

const INSTRUMENTATION_SCOPE_ATTRIBUTES_LABEL: sval::Label =
    sval::Label::new("attributes").with_tag(&sval::tags::VALUE_IDENT);

const INSTRUMENTATION_SCOPE_ATTRIBUTES_INDEX: sval::Index = sval::Index::new(1);

#[derive(Value)]
pub struct InlineInstrumentationScopeAttributes<'a> {
    #[sval(label = INSTRUMENTATION_SCOPE_ATTRIBUTES_LABEL, index = INSTRUMENTATION_SCOPE_ATTRIBUTES_INDEX)]
    pub attributes: &'a [KeyValue<&'a str, &'a AnyValue<'a>>],
}

pub struct PropsInstrumentationScopeAttributes<P>(pub P);

impl<P: emit::props::Props> sval::Value for PropsInstrumentationScopeAttributes<P> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.record_tuple_begin(None, None, None, None)?;

        stream_field(
            &mut *stream,
            &INSTRUMENTATION_SCOPE_ATTRIBUTES_LABEL,
            &INSTRUMENTATION_SCOPE_ATTRIBUTES_INDEX,
            |stream| stream_attributes(stream, &self.0, |_, _| false),
        )?;

        stream.record_tuple_end(None, None, None)
    }
}
