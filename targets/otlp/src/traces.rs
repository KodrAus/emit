use emit_batcher::BatchError;

use crate::data::{
    self,
    traces::{ExportTraceServiceRequest, PropsSpanAttributes, ResourceSpans, ScopeSpans, Span},
    PreEncoded,
};

pub(crate) fn encode_event(
    evt: &emit_core::event::Event<impl emit_core::props::Props>,
) -> Option<PreEncoded> {
    let (start_time_unix_nano, end_time_unix_nano) = evt
        .extent()
        .and_then(|extent| extent.as_span())
        .map(|span| {
            (
                span.start.to_unix_time().as_nanos() as u64,
                span.end.to_unix_time().as_nanos() as u64,
            )
        })?;

    let protobuf = sval_protobuf::stream_to_protobuf(Span {
        start_time_unix_nano,
        end_time_unix_nano,
        name: &Some(data::AnyValue::<_, (), (), ()>::String(
            &sval::Display::new(evt.tpl()),
        )),
        attributes: &PropsSpanAttributes {
            time_unix_nano: end_time_unix_nano,
            props: evt.props(),
        },
        dropped_attributes_count: 0,
        kind: data::traces::SpanKind::Unspecified,
    });

    Some(PreEncoded::Proto(protobuf))
}

pub(crate) fn encode_request(
    resource: Option<&PreEncoded>,
    scope: Option<&PreEncoded>,
    spans: &[PreEncoded],
) -> Result<PreEncoded, BatchError<Vec<PreEncoded>>> {
    Ok(PreEncoded::Proto(sval_protobuf::stream_to_protobuf(
        ExportTraceServiceRequest {
            resource_spans: &[ResourceSpans {
                resource: &resource,
                scope_spans: &[ScopeSpans {
                    scope: &scope,
                    spans,
                    schema_url: "",
                }],
                schema_url: "",
            }],
        },
    )))
}
