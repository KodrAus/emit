use emit_batcher::BatchError;
use std::{future::Future, ops::ControlFlow, sync::Arc, time::Duration};

use crate::data::PreEncoded;

pub(super) struct OtlpClient<T> {
    sender: emit_batcher::Sender<T>,
}

pub(super) struct OtlpClientBuilder {
    dst: Destination,
}

enum Destination {
    HttpProto {
        resource: Option<PreEncoded>,
        scope: Option<PreEncoded>,
        url: String,
    },
}

impl<T> OtlpClient<T> {
    pub fn emit(&self, value: T) {
        self.sender.send(value);
    }

    pub fn blocking_flush(&self, timeout: Duration) {
        emit_batcher::tokio::blocking_flush(&self.sender, timeout)
    }
}

impl OtlpClientBuilder {
    pub fn http_proto(dst: impl Into<String>) -> Self {
        OtlpClientBuilder {
            dst: Destination::HttpProto {
                url: dst.into(),
                resource: None,
                scope: None,
            },
        }
    }

    pub fn resource(mut self, resource: impl emit_core::props::Props) -> Self {
        /*
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
        */

        self
    }

    pub fn spawn<
        T: Send + 'static,
        F: Future<Output = Result<(), BatchError<T>>> + Send + 'static,
    >(
        self,
        mut on_batch: impl FnMut(OtlpSender, Vec<T>) -> F + Send + 'static,
    ) -> OtlpClient<T> {
        let (sender, receiver) = emit_batcher::bounded(1024);

        let client = OtlpSender {
            client: Arc::new(match self.dst {
                Destination::HttpProto {
                    url,
                    resource,
                    scope,
                } => RawClient::HttpProto {
                    url,
                    resource,
                    scope,
                    client: reqwest::Client::new(),
                },
            }),
        };

        emit_batcher::tokio::spawn(receiver, move |batch| on_batch(client.clone(), batch));

        OtlpClient { sender }
    }
}

#[derive(Clone)]
pub struct OtlpSender {
    client: Arc<RawClient>,
}

enum RawClient {
    HttpProto {
        url: String,
        resource: Option<PreEncoded>,
        scope: Option<PreEncoded>,
        client: reqwest::Client,
    },
}

impl OtlpSender {
    pub async fn emit<T>(
        self,
        batch: Vec<T>,
        // TODO: Encode proto
        encode: impl FnOnce(
            Option<&PreEncoded>,
            Option<&PreEncoded>,
            &[T],
        ) -> Result<Vec<u8>, BatchError<T>>,
    ) -> Result<(), BatchError<T>> {
        match *self.client {
            RawClient::HttpProto {
                ref url,
                ref resource,
                ref scope,
                ref client,
            } => {
                let body = encode(resource.as_ref(), scope.as_ref(), &batch)?;

                client
                    .request(reqwest::Method::POST, url)
                    .header("content-type", "application/x-protobuf")
                    .body(body)
                    .send()
                    .await
                    .map_err(|e| BatchError::retry(e, batch))?;
            }
        }

        Ok(())
    }
}
