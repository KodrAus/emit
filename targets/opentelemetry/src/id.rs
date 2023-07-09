use opentelemetry_api::trace::{SpanId, TraceId};

pub(crate) fn to_trace_id(id: emit_core::id::Id) -> TraceId {
    TraceId::from_bytes(
        id.trace()
            .map(|id| id.to_u128())
            .unwrap_or_default()
            .to_be_bytes(),
    )
}

pub(crate) fn to_span_id(id: emit_core::id::Id) -> SpanId {
    SpanId::from_bytes(
        id.span()
            .map(|id| id.to_u64())
            .unwrap_or_default()
            .to_be_bytes(),
    )
}

pub(crate) fn from_trace_span_ids(trace: TraceId, span: SpanId) -> emit_core::id::Id {
    let trace_id = u128::from_be_bytes(trace.to_bytes());
    let span_id = u64::from_be_bytes(span.to_bytes());

    emit_core::id::Id::new(
        Some(emit_core::id::TraceId::from_u128(trace_id)),
        Some(emit_core::id::SpanId::from_u64(span_id)),
    )
}
