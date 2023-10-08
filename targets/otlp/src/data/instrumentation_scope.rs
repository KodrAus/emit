use sval_derive::Value;

use super::{attributes::stream_attributes, AnyValue, KeyValue};

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

pub struct EmitInstrumentationScopeAttributes<P>(pub P);

impl<P: emit_core::props::Props> sval::Value for EmitInstrumentationScopeAttributes<P> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.record_tuple_begin(None, None, None, None)?;

        stream.record_tuple_value_begin(
            None,
            &INSTRUMENTATION_SCOPE_ATTRIBUTES_LABEL,
            &INSTRUMENTATION_SCOPE_ATTRIBUTES_INDEX,
        )?;
        stream_attributes(&mut *stream, &self.0, |_, _| false)?;
        stream.record_tuple_value_end(
            None,
            &INSTRUMENTATION_SCOPE_ATTRIBUTES_LABEL,
            &INSTRUMENTATION_SCOPE_ATTRIBUTES_INDEX,
        )?;

        stream.record_tuple_end(None, None, None)
    }
}
