mod export_logs_service;
mod log_record;

use emit_batcher::BatchError;

pub use self::{export_logs_service::*, log_record::*};

use super::{
    default_message_formatter, AnyValue, Encoding, MessageFormatter, MessageRenderer, PreEncoded,
};

pub(crate) struct EventEncoder {
    pub body: Box<MessageFormatter>,
}

impl Default for EventEncoder {
    fn default() -> Self {
        EventEncoder {
            body: default_message_formatter(),
        }
    }
}

impl EventEncoder {
    pub(crate) fn encode_event<E: Encoding>(
        &self,
        evt: &emit::event::Event<impl emit::props::Props>,
    ) -> PreEncoded {
        let time_unix_nano = evt
            .extent()
            .map(|extent| extent.as_point().to_unix_time().as_nanos() as u64)
            .unwrap_or_default();

        let observed_time_unix_nano = time_unix_nano;

        E::encode(LogRecord {
            time_unix_nano,
            observed_time_unix_nano,
            body: &Some(AnyValue::<_, (), (), ()>::String(&sval::Display::new(
                MessageRenderer {
                    fmt: &self.body,
                    evt,
                },
            ))),
            attributes: &PropsLogRecordAttributes::<E::TraceId, E::SpanId, _>::new(evt.props()),
            dropped_attributes_count: 0,
            flags: Default::default(),
        })
    }
}

pub(crate) fn encode_request<E: Encoding>(
    resource: Option<&PreEncoded>,
    scope: Option<&PreEncoded>,
    log_records: &[PreEncoded],
) -> Result<PreEncoded, BatchError<Vec<PreEncoded>>> {
    Ok(E::encode(ExportLogsServiceRequest {
        resource_logs: &[ResourceLogs {
            resource: &resource,
            scope_logs: &[ScopeLogs {
                scope: &scope,
                log_records,
                schema_url: "",
            }],
            schema_url: "",
        }],
    }))
}

#[cfg(feature = "decode_responses")]
pub(crate) fn decode_response(body: Result<&[u8], &[u8]>) {
    match body {
        Ok(body) => {
            emit::debug!(
                rt: emit::runtime::internal(),
                "received logs {response}",
                #[emit::as_debug]
                response: crate::data::generated::response::decode::<crate::data::generated::collector::logs::v1::ExportLogsServiceResponse>(body),
            );
        }
        Err(body) => {
            emit::warn!(
                rt: emit::runtime::internal(),
                "received logs {response}",
                #[emit::as_debug]
                response: crate::data::generated::response::decode::<crate::data::generated::google::rpc::Status>(body),
            );
        }
    }
}
