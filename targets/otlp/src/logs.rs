use crate::{
    client::{OtlpClient, OtlpClientBuilder},
    data,
};
use std::time::Duration;
use sval_protobuf::buf::ProtoBuf;

pub fn http_proto(dst: impl Into<String>) -> OtlpLogsEmitterBuilder {
    OtlpLogsEmitterBuilder {
        inner: OtlpClientBuilder::http_proto(dst),
    }
}

pub struct OtlpLogsEmitterBuilder {
    inner: OtlpClientBuilder,
}

pub struct OtlpLogsEmitter {
    inner: OtlpClient<ProtoBuf>,
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
                client.emit(batch, |ref resource, ref scope, batch| {
                    let protobuf =
                        sval_protobuf::stream_to_protobuf(data::ExportLogsServiceRequest {
                            resource_logs: &[data::ResourceLogs {
                                resource,
                                scope_logs: &[data::ScopeLogs {
                                    scope,
                                    log_records: batch,
                                    schema_url: "",
                                }],
                                schema_url: "",
                            }],
                        });

                    Ok(protobuf.to_vec().into_owned())
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

        let protobuf = sval_protobuf::stream_to_protobuf(data::LogRecord {
            time_unix_nano,
            observed_time_unix_nano,
            body: &Some(data::AnyValue::<_, (), (), ()>::String(
                &sval::Display::new(evt.msg()),
            )),
            attributes: &data::EmitLogRecordAttributes(evt.props()),
            dropped_attributes_count: 0,
            flags: Default::default(),
        });

        self.inner.emit(protobuf)
    }

    fn blocking_flush(&self, timeout: Duration) {
        self.inner.blocking_flush(timeout)
    }
}
