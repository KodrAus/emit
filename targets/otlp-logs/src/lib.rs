use std::{ops::ControlFlow, sync::Arc};

use batcher::Batcher;
use otlp::{
    collector::logs::v1::ExportLogsServiceRequest,
    common::v1::{InstrumentationScope, KeyValue},
    logs::v1::{LogRecord, ResourceLogs, ScopeLogs},
    resource::v1::Resource,
};

mod batcher;
mod otlp;
mod value;

pub fn http(dst: impl Into<String>) -> OtlpTargetBuilder {
    OtlpTargetBuilder {
        resource: None,
        scope: None,
        dst: Destination::HttpProto(dst.into()),
    }
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct OtlpTarget {
    batcher: Batcher<LogRecord>,
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
    pub fn resource(mut self, resource: impl emit::Props) -> Self {
        let mut attributes = Vec::new();

        resource.for_each(|k, v| {
            let key = k.to_string();
            let value = value::to_value(v);

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
        let batcher = Batcher::new(1024);

        tokio::spawn({
            let receiver = batcher.receiver();

            async move {
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

                receiver.exec(move |batch| client.clone().emit(batch)).await
            }
        });

        OtlpTarget { batcher }
    }
}

impl OtlpTarget {
    pub async fn flush(&self) {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        self.batcher.watch_next_flush(move || {
            let _ = sender.send(());
        });

        let _ = receiver.await;
    }
}

impl emit::Target for OtlpTarget {
    fn emit_event<P: emit::Props>(&self, evt: &emit::Event<P>) {
        let record = value::to_record(evt);

        self.batcher.send(record);
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let request = ExportLogsServiceRequest {
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
                request.encode(&mut buf)?;

                client
                    .request(reqwest::Method::POST, url)
                    .header("content-type", "application/x-protobuf")
                    .body(buf)
                    .send()
                    .await?;
            }
        }

        Ok(())
    }
}
