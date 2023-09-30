use sval_derive::Value;

use super::{AnyValue, KeyValue};

#[derive(Value)]
pub struct InstrumentationScope<'a, A: ?Sized = InlineInstrumentationScopeAttributes<'a>> {
    #[sval(index = 1)]
    pub name: &'a str,
    #[sval(index = 2)]
    pub version: &'a str,
    #[sval(flatten)]
    pub attributes: &'a A,
    #[sval(index = 4)]
    pub dropped_attribute_count: u32,
}

#[derive(Value)]
pub struct InlineInstrumentationScopeAttributes<'a> {
    #[sval(index = 1)]
    pub attributes: &'a [KeyValue<&'a str, &'a AnyValue<'a>>],
}
