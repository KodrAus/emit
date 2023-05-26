use std::{ops::ControlFlow, sync::Arc, time::Duration};

use proto::{
    collector::logs::v1::ExportLogsServiceRequest,
    common::v1::{InstrumentationScope, KeyValue},
    logs::v1::{LogRecord, ResourceLogs, ScopeLogs},
    resource::v1::Resource,
};

mod proto;
mod record;

pub fn http(dst: impl Into<String>) -> OtlpTargetBuilder {
    OtlpTargetBuilder {
        resource: None,
        scope: None,
        dst: Destination::HttpProto(dst.into()),
    }
}

pub struct OtlpTarget {
    sender: emit_batcher::Sender<LogRecord>,
}

pub struct OtlpTargetBuilder {
    resource: Option<Resource>,
    scope: Option<InstrumentationScope>,
    dst: Destination,
}

enum Destination {
    HttpProto(String),
}

impl OtlpTargetBuilder {
    pub fn resource(mut self, resource: impl emit_core::props::Props) -> Self {
        let mut attributes = Vec::new();

        resource.for_each(|k, v| {
            let key = k.to_string();
            let value = record::to_value(v);

            attributes.push(KeyValue { key, value });

            ControlFlow::Continue(())
        });

        self.resource = Some(Resource {
            attributes,
            dropped_attributes_count: 0,
        });

        self
    }

    pub fn spawn(self) -> OtlpTarget {
        let (sender, receiver) = emit_batcher::bounded(1024);

        let client = OtlpClient {
            resource: self.resource,
            scope: self.scope,
            client: Arc::new(match self.dst {
                Destination::HttpProto(url) => Client::HttpProto {
                    url,
                    client: reqwest::Client::new(),
                },
            }),
        };

        emit_batcher::tokio::spawn(receiver, move |batch| client.clone().emit(batch));

        OtlpTarget { sender }
    }
}

impl emit_core::target::Target for OtlpTarget {
    fn emit_event<P: emit_core::props::Props>(&self, evt: &emit_core::event::Event<P>) {
        let record = record::to_record(evt);

        // Non-blocking
        self.sender.send(record);
    }

    fn blocking_flush(&self, timeout: Duration) {
        emit_batcher::tokio::blocking_flush(&self.sender, timeout)
    }
}

#[derive(Clone)]
struct OtlpClient {
    resource: Option<Resource>,
    scope: Option<InstrumentationScope>,
    client: Arc<Client>,
}

enum Client {
    HttpProto {
        url: String,
        client: reqwest::Client,
    },
}

impl OtlpClient {
    pub async fn emit(
        self,
        batch: Vec<LogRecord>,
    ) -> Result<(), emit_batcher::BatchError<LogRecord>> {
        let mut request = ExportLogsServiceRequest {
            resource_logs: vec![ResourceLogs {
                resource: self.resource,
                scope_logs: vec![ScopeLogs {
                    scope: self.scope,
                    log_records: batch,
                    schema_url: String::new(),
                }],
                schema_url: String::new(),
            }],
        };

        match *self.client {
            Client::HttpProto {
                ref url,
                ref client,
            } => {
                use prost::Message;

                let mut buf = Vec::new();
                request
                    .encode(&mut buf)
                    .map_err(emit_batcher::BatchError::no_retry)?;

                client
                    .request(reqwest::Method::POST, url)
                    .header("content-type", "application/x-protobuf")
                    .body(buf.clone())
                    .send()
                    .await
                    .map_err(|e| {
                        emit_batcher::BatchError::retry(
                            e,
                            request
                                .resource_logs
                                .pop()
                                .unwrap()
                                .scope_logs
                                .pop()
                                .unwrap()
                                .log_records,
                        )
                    })?;
            }
        }

        Ok(())
    }
}
