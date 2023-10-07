use sval_derive::Value;

use super::{AnyValue, KeyValue};

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

#[derive(Value)]
pub struct InlineInstrumentationScopeAttributes<'a> {
    #[sval(label = "attributes", index = 1)]
    pub attributes: &'a [KeyValue<&'a str, &'a AnyValue<'a>>],
}
