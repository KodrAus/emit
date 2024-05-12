mod export_trace_service;
mod span;

use emit::{well_known::KEY_SPAN_NAME, Filter, Props};
use emit_batcher::BatchError;

pub use self::{export_trace_service::*, span::*};

use super::{
    EventEncoder, MessageFormatter, MessageRenderer, PreEncoded, RawEncoder, RequestEncoder,
};

pub(crate) struct TracesEventEncoder {
    pub name: Box<MessageFormatter>,
}

impl Default for TracesEventEncoder {
    fn default() -> Self {
        TracesEventEncoder {
            name: default_name_formatter(),
        }
    }
}

fn default_name_formatter() -> Box<MessageFormatter> {
    Box::new(|evt, f| {
        if let Some(name) = evt.props().get(KEY_SPAN_NAME) {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", evt.msg())
        }
    })
}

impl EventEncoder for TracesEventEncoder {
    fn encode_event<E: RawEncoder>(
        &self,
        evt: &emit::event::Event<impl emit::props::Props>,
    ) -> Option<PreEncoded> {
        if !emit::kind::is_span_filter().matches(evt) {
            return None;
        }

        let (start_time_unix_nano, end_time_unix_nano) = evt
            .extent()
            .filter(|extent| extent.is_span())
            .map(|extent| {
                (
                    extent.as_range().start.to_unix().as_nanos() as u64,
                    extent.as_range().end.to_unix().as_nanos() as u64,
                )
            })?;

        Some(E::encode(Span {
            start_time_unix_nano,
            end_time_unix_nano,
            name: &sval::Display::new(MessageRenderer {
                fmt: &self.name,
                evt,
            }),
            attributes: &PropsSpanAttributes::<E::TraceId, E::SpanId, _>::new(
                end_time_unix_nano,
                evt.props(),
            ),
            dropped_attributes_count: 0,
            kind: SpanKind::Unspecified,
        }))
    }
}

#[derive(Default)]
pub(crate) struct TracesRequestEncoder;

impl RequestEncoder for TracesRequestEncoder {
    fn encode_request<E: RawEncoder>(
        &self,
        resource: Option<&PreEncoded>,
        scope: Option<&PreEncoded>,
        items: &[PreEncoded],
    ) -> Result<PreEncoded, BatchError<Vec<PreEncoded>>> {
        Ok(E::encode(ExportTraceServiceRequest {
            resource_spans: &[ResourceSpans {
                resource: &resource,
                scope_spans: &[ScopeSpans {
                    scope: &scope,
                    spans: items,
                    schema_url: "",
                }],
                schema_url: "",
            }],
        }))
    }
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
