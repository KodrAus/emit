mod export_logs_service;
mod log_record;

use emit_batcher::BatchError;

pub use self::{export_logs_service::*, log_record::*};

use super::{AnyValue, MessageFormatter, MessageRenderer, PreEncoded};

pub(crate) struct EventEncoder {
    pub body: Box<MessageFormatter>,
}

impl EventEncoder {
    pub(crate) fn encode_event(
        &self,
        evt: &emit_core::event::Event<impl emit_core::props::Props>,
    ) -> PreEncoded {
        let time_unix_nano = evt
            .extent()
            .map(|extent| extent.to_point().to_unix_time().as_nanos() as u64)
            .unwrap_or_default();

        let observed_time_unix_nano = time_unix_nano;

        let protobuf = sval_protobuf::stream_to_protobuf(LogRecord {
            time_unix_nano,
            observed_time_unix_nano,
            body: &Some(AnyValue::<_, (), (), ()>::String(&sval::Display::new(
                MessageRenderer {
                    fmt: &self.body,
                    evt,
                },
            ))),
            attributes: &PropsLogRecordAttributes(evt.props()),
            dropped_attributes_count: 0,
            flags: Default::default(),
        });

        PreEncoded::Proto(protobuf)
    }
}

pub(crate) fn encode_request(
    resource: Option<&PreEncoded>,
    scope: Option<&PreEncoded>,
    log_records: &[PreEncoded],
) -> Result<PreEncoded, BatchError<Vec<PreEncoded>>> {
    Ok(PreEncoded::Proto(sval_protobuf::stream_to_protobuf(
        ExportLogsServiceRequest {
            resource_logs: &[ResourceLogs {
                resource: &resource,
                scope_logs: &[ScopeLogs {
                    scope: &scope,
                    log_records,
                    schema_url: "",
                }],
                schema_url: "",
            }],
        },
    )))
}
