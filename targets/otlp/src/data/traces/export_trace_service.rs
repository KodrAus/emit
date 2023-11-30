use sval_derive::Value;

use crate::data::{InstrumentationScope, Resource};

use super::Span;

#[derive(Value)]
pub struct ExportTraceServiceRequest<'a, RL: ?Sized = [ResourceSpans<'a>]> {
    #[sval(label = "resourceSpans", index = 1)]
    pub resource_spans: &'a RL,
}

#[derive(Value)]
pub struct ResourceSpans<'a, R: ?Sized = Resource<'a>, SL: ?Sized = [ScopeSpans<'a>]> {
    #[sval(label = "resource", index = 1)]
    pub resource: &'a R,
    #[sval(label = "scopeSpans", index = 2)]
    pub scope_spans: &'a SL,
    #[sval(label = "schemaUrl", index = 3)]
    pub schema_url: &'a str,
}

#[derive(Value)]
pub struct ScopeSpans<'a, IS: ?Sized = InstrumentationScope<'a>, LR: ?Sized = &'a [Span<'a>]> {
    #[sval(label = "scope", index = 1)]
    pub scope: &'a IS,
    #[sval(label = "spans", index = 2)]
    pub spans: &'a LR,
    #[sval(label = "schemaUrl", index = 3)]
    pub schema_url: &'a str,
}
