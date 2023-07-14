use crate::{
    proto::{
        common::v1::{InstrumentationScope, KeyValue},
        resource::v1::Resource,
    },
    value,
};
use emit_batcher::BatchError;
use std::{future::Future, ops::ControlFlow, sync::Arc, time::Duration};

pub(super) struct OtlpClient<T> {
    sender: emit_batcher::Sender<T>,
}

pub(super) struct OtlpClientBuilder {
    resource: Option<Resource>,
    scope: Option<InstrumentationScope>,
    dst: Destination,
}

enum Destination {
    HttpProto(String),
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
    pub fn http(dst: impl Into<String>) -> Self {
        OtlpClientBuilder {
            resource: None,
            scope: None,
            dst: Destination::HttpProto(dst.into()),
        }
    }

    pub fn resource(mut self, resource: impl emit_core::props::Props) -> Self {
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

    pub fn spawn<
        T: Send + 'static,
        F: Future<Output = Result<(), BatchError<T>>> + Send + 'static,
    >(
        self,
        mut on_batch: impl FnMut(OtlpSender, Vec<T>) -> F + Send + 'static,
    ) -> OtlpClient<T> {
        let (sender, receiver) = emit_batcher::bounded(1024);

        let client = OtlpSender {
            resource: self.resource,
            scope: self.scope,
            client: Arc::new(match self.dst {
                Destination::HttpProto(url) => RawClient::HttpProto {
                    url,
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
    resource: Option<Resource>,
    scope: Option<InstrumentationScope>,
    client: Arc<RawClient>,
}

enum RawClient {
    HttpProto {
        url: String,
        client: reqwest::Client,
    },
}

impl OtlpSender {
    pub async fn emit<T>(
        self,
        batch: Vec<T>,
        encode: impl FnOnce(
            Option<Resource>,
            Option<InstrumentationScope>,
            &[T],
        ) -> Result<Vec<u8>, BatchError<T>>,
    ) -> Result<(), BatchError<T>> {
        let body = encode(self.resource, self.scope, &batch)?;

        match *self.client {
            RawClient::HttpProto {
                ref url,
                ref client,
            } => {
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
