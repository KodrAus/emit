use crate::{
    client::{OtlpClient, OtlpClientBuilder},
    proto::{
        collector::logs::v1::ExportLogsServiceRequest,
        common::v1::{any_value::Value, AnyValue, KeyValue},
        logs::v1::{LogRecord, ResourceLogs, ScopeLogs, SeverityNumber},
    },
    value::to_value,
};
use emit_batcher::BatchError;
use emit_core::well_known;
use std::{collections::HashSet, ops::ControlFlow, time::Duration};

pub fn http(dst: impl Into<String>) -> OtlpLogsEmitterBuilder {
    OtlpLogsEmitterBuilder {
        inner: OtlpClientBuilder::http(dst),
    }
}

pub struct OtlpLogsEmitterBuilder {
    inner: OtlpClientBuilder,
}

pub struct OtlpLogsEmitter {
    inner: OtlpClient<LogRecord>,
}

impl OtlpLogsEmitterBuilder {
    pub fn resource(self, resource: impl emit_core::props::Props) -> Self {
        OtlpLogsEmitterBuilder {
            inner: self.inner.resource(resource),
        }
    }

    pub fn spawn(self) -> OtlpLogsEmitter {
        OtlpLogsEmitter {
            inner: self.inner.spawn(|client, batch| {
                client.emit(batch, |resource, scope, batch| {
                    use prost::Message;

                    let request = ExportLogsServiceRequest {
                        resource_logs: vec![ResourceLogs {
                            resource,
                            scope_logs: vec![ScopeLogs {
                                scope,
                                log_records: batch.to_vec(),
                                schema_url: String::new(),
                            }],
                            schema_url: String::new(),
                        }],
                    };

                    let mut buf = Vec::new();
                    request.encode(&mut buf).map_err(BatchError::no_retry)?;

                    Ok(buf)
                })
            }),
        }
    }
}

impl emit_core::emitter::Emitter for OtlpLogsEmitter {
    fn emit<P: emit_core::props::Props>(&self, evt: &emit_core::event::Event<P>) {
        let time_unix_nano = evt
            .extent()
            .as_point()
            .map(|ts| ts.as_unix_time().as_nanos() as u64)
            .unwrap_or_default();

        let observed_time_unix_nano = time_unix_nano;

        let body = Some(AnyValue {
            value: Some(Value::StringValue(evt.msg().to_string())),
        });

        let mut level = emit_core::level::Level::default();
        let mut attributes = Vec::new();
        let mut trace_id = Vec::new();
        let mut span_id = Vec::new();

        let mut seen = HashSet::new();
        evt.props().for_each(|k, v| {
            match k.as_str() {
                well_known::LVL_KEY => {
                    level = v.to_level().unwrap_or_default();
                }
                well_known::SPAN_ID_KEY => {
                    span_id = v
                        .to_span_id()
                        .map(|span_id| span_id.to_hex().to_vec())
                        .unwrap_or_default();
                }
                well_known::TRACE_ID_KEY => {
                    trace_id = v
                        .to_trace_id()
                        .map(|trace_id| trace_id.to_hex().to_vec())
                        .unwrap_or_default();
                }
                _ => {
                    let key = k.to_string();

                    if seen.insert(k) {
                        let value = to_value(v);

                        attributes.push(KeyValue { key, value });
                    }
                }
            }

            ControlFlow::Continue(())
        });

        let severity_number = match level {
            emit_core::level::Level::Debug => SeverityNumber::Debug as i32,
            emit_core::level::Level::Info => SeverityNumber::Info as i32,
            emit_core::level::Level::Warn => SeverityNumber::Warn as i32,
            emit_core::level::Level::Error => SeverityNumber::Error as i32,
        };

        let severity_text = level.to_string();

        self.inner.emit(LogRecord {
            time_unix_nano,
            observed_time_unix_nano,
            severity_number,
            severity_text,
            body,
            attributes,
            dropped_attributes_count: 0,
            flags: Default::default(),
            trace_id,
            span_id,
        })
    }

    fn blocking_flush(&self, timeout: Duration) {
        self.inner.blocking_flush(timeout)
    }
}
