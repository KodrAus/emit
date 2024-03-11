mod export_trace_service;
mod span;

use emit_batcher::BatchError;

pub use self::{export_trace_service::*, span::*};

use super::{MessageFormatter, MessageRenderer, PreEncoded};

pub(crate) struct EventEncoder {
    pub name: Box<MessageFormatter>,
}

impl EventEncoder {
    pub(crate) fn encode_event(
        &self,
        evt: &emit::event::Event<impl emit::props::Props>,
    ) -> Option<PreEncoded> {
        let (start_time_unix_nano, end_time_unix_nano) = evt
            .extent()
            .filter(|extent| extent.is_span())
            .map(|extent| {
                (
                    extent.as_range().start.to_unix_time().as_nanos() as u64,
                    extent.as_range().end.to_unix_time().as_nanos() as u64,
                )
            })?;

        let protobuf = sval_protobuf::stream_to_protobuf(Span {
            start_time_unix_nano,
            end_time_unix_nano,
            name: &sval::Display::new(MessageRenderer {
                fmt: &self.name,
                evt,
            }),
            attributes: &PropsSpanAttributes {
                time_unix_nano: end_time_unix_nano,
                props: evt.props(),
            },
            dropped_attributes_count: 0,
            kind: SpanKind::Unspecified,
        });

        Some(PreEncoded::Proto(protobuf))
    }
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

#[cfg(feature = "decode_responses")]
pub(crate) fn decode_response(body: Result<&[u8], &[u8]>) {
    match body {
        Ok(body) => {
            emit::debug!(
                rt: emit::runtime::internal(),
                "received traces {response}",
                #[emit::as_debug]
                response: crate::data::generated::response::decode::<crate::data::generated::collector::trace::v1::ExportTraceServiceResponse>(body),
            );
        }
        Err(body) => {
            emit::warn!(
                rt: emit::runtime::internal(),
                "received traces {response}",
                #[emit::as_debug]
                response: crate::data::generated::response::decode::<crate::data::generated::google::rpc::Status>(body),
            );
        }
    }
}
