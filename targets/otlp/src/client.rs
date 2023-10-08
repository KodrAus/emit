use emit_batcher::BatchError;
use std::{future::Future, sync::Arc, time::Duration};
use sval_protobuf::buf::ProtoBuf;

use crate::{
    data::{self, PreEncoded},
    Error,
};

use self::http::HttpConnection;

mod http;

pub(super) struct OtlpClient<T> {
    sender: emit_batcher::Sender<Vec<T>>,
}

pub(super) struct OtlpClientBuilder {
    dst: Destination,
}

enum Destination {
    HttpProto {
        resource: Option<ProtoBuf>,
        scope: Option<ProtoBuf>,
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

    pub fn resource(mut self, attributes: impl emit_core::props::Props) -> Self {
        match self.dst {
            Destination::HttpProto {
                ref mut resource, ..
            } => {
                let protobuf = sval_protobuf::stream_to_protobuf(data::Resource {
                    attributes: &data::EmitResourceAttributes(attributes),
                    dropped_attribute_count: 0,
                });

                *resource = Some(protobuf);
            }
        }

        self
    }

    pub fn scope(
        mut self,
        name: &str,
        version: &str,
        attributes: impl emit_core::props::Props,
    ) -> Self {
        match self.dst {
            Destination::HttpProto { ref mut scope, .. } => {
                let protobuf = sval_protobuf::stream_to_protobuf(data::InstrumentationScope {
                    name,
                    version,
                    attributes: &data::EmitInstrumentationScopeAttributes(attributes),
                    dropped_attribute_count: 0,
                });

                *scope = Some(protobuf);
            }
        }

        self
    }

    pub fn spawn<
        T: Send + 'static,
        F: Future<Output = Result<(), BatchError<Vec<T>>>> + Send + 'static,
    >(
        self,
        mut on_batch: impl FnMut(OtlpSender, Vec<T>) -> F + Send + 'static,
    ) -> Result<OtlpClient<T>, Error> {
        let (sender, receiver) = emit_batcher::bounded(10_000);

        let client = OtlpSender {
            client: Arc::new(match self.dst {
                Destination::HttpProto {
                    url,
                    resource,
                    scope,
                } => RawClient::HttpProto {
                    http: HttpConnection::new(&url)?,
                    resource: resource.map(PreEncoded::Proto),
                    scope: scope.map(PreEncoded::Proto),
                },
            }),
        };

        emit_batcher::tokio::spawn(receiver, move |batch| on_batch(client.clone(), batch));

        Ok(OtlpClient { sender })
    }
}

#[derive(Clone)]
pub struct OtlpSender {
    client: Arc<RawClient>,
}

enum RawClient {
    HttpProto {
        http: HttpConnection,
        resource: Option<PreEncoded>,
        scope: Option<PreEncoded>,
    },
}

impl OtlpSender {
    pub(crate) async fn emit<T>(
        self,
        batch: Vec<T>,
        encode: impl FnOnce(
            Option<&PreEncoded>,
            Option<&PreEncoded>,
            &[T],
        ) -> Result<PreEncoded, BatchError<Vec<T>>>,
    ) -> Result<(), BatchError<Vec<T>>> {
        match *self.client {
            RawClient::HttpProto {
                ref http,
                ref resource,
                ref scope,
            } => {
                http.send(encode(resource.as_ref(), scope.as_ref(), &batch)?)
                    .await
                    .map_err(|e| BatchError::no_retry(e))?;
            }
        }

        Ok(())
    }
}
