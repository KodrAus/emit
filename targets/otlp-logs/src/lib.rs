use std::{
    cmp,
    ops::ControlFlow,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

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
    pub fn resource(mut self, resource: impl emit::Props) -> Self {
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

        let target = async move {
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

            receiver
                .exec(
                    |delay| tokio::time::sleep(delay),
                    move |batch| client.clone().emit(batch),
                )
                .await
        };

        match tokio::runtime::Handle::try_current() {
            // If we're in a current runtime then spawn on it
            Ok(handle) => {
                handle.spawn(target);
            }
            // If we're not in a current runtime then spawn a
            // background thread and run the work there
            Err(_) => {
                thread::spawn(move || {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(target)
                        .unwrap();
                });
            }
        }

        OtlpTarget { sender }
    }
}

impl emit::target::Target for OtlpTarget {
    fn emit_event<P: emit::Props>(&self, evt: &emit::Event<P>) {
        let record = record::to_record(evt);

        // Non-blocking
        self.sender.send(record);
    }

    fn blocking_flush(&self, timeout: Duration) {
        tokio::task::block_in_place(|| {
            let (sender, mut receiver) = tokio::sync::oneshot::channel();

            self.sender.on_next_flush(move || {
                let _ = sender.send(());
            });

            // If there's nothing to flush then return immediately
            if receiver.try_recv().is_ok() {
                return;
            }

            match tokio::runtime::Handle::try_current() {
                // If we're on a `tokio` thread then await the receiver
                Ok(handle) => handle.block_on(async {
                    let _ = tokio::time::timeout(timeout, receiver).await;
                }),
                // If we're not on a `tokio` thread then wait for
                // a notification
                Err(_) => {
                    let now = Instant::now();
                    let mut wait = Duration::from_micros(1);

                    while now.elapsed() < timeout {
                        if receiver.try_recv().is_ok() {
                            return;
                        }

                        thread::sleep(wait);
                        wait += cmp::min(wait * 2, timeout / 2);
                    }
                }
            }
        });
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
