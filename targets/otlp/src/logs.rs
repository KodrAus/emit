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
                client.emit(batch, |resource, scope, batch| {
                    Ok(
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
                        })
                        .to_vec()
                        .into_owned(),
                    )
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

        println!("{}", sval_json::stream_to_string(data::LogRecord {
            time_unix_nano,
            observed_time_unix_nano,
            body: Some(data::DisplayValue::String(sval::Display::new(evt.msg()))),
            attributes: data::PropsLogRecordAttributes(evt.props()),
            dropped_attributes_count: 0,
            flags: Default::default(),
        }).unwrap());

        println!("{}", protoscope(&sval_protobuf::stream_to_protobuf(data::LogRecord {
            time_unix_nano,
            observed_time_unix_nano,
            body: Some(data::DisplayValue::String(sval::Display::new(evt.msg()))),
            attributes: data::PropsLogRecordAttributes(evt.props()),
            dropped_attributes_count: 0,
            flags: Default::default(),
        }).to_vec()));

        self.inner
            .emit(sval_protobuf::stream_to_protobuf(data::LogRecord {
                time_unix_nano,
                observed_time_unix_nano,
                body: Some(data::DisplayValue::String(sval::Display::new(evt.msg()))),
                attributes: data::PropsLogRecordAttributes(evt.props()),
                dropped_attributes_count: 0,
                flags: Default::default(),
            }))
    }

    fn blocking_flush(&self, timeout: Duration) {
        self.inner.blocking_flush(timeout)
    }
}

fn protoscope(encoded: &[u8]) -> String {
    use std::{
        io::{Read, Write},
        process::{Command, Stdio},
    };

    let mut protoscope = Command::new("protoscope")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to call protoscope");

    let mut stdin = protoscope.stdin.take().expect("missing stdin");
    stdin.write_all(encoded).expect("failed to write");
    drop(stdin);

    let mut buf = String::new();
    protoscope
        .stdout
        .take()
        .expect("missing stdout")
        .read_to_string(&mut buf)
        .expect("failed to read");

    buf
}
