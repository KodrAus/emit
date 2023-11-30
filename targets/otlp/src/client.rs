use emit_batcher::BatchError;
use std::{sync::Arc, time::Duration};

use crate::{
    data::{self, logs, traces, PreEncoded},
    Error,
};

use self::http::HttpConnection;

mod http;

pub struct OtlpClient {
    emit_logs: bool,
    emit_traces: bool,
    sender: emit_batcher::Sender<Channel<PreEncoded>>,
}

impl emit_core::emitter::Emitter for OtlpClient {
    fn emit<P: emit_core::props::Props>(&self, evt: &emit_core::event::Event<P>) {
        if self.emit_traces {
            if let Some(encoded) = traces::encode_event(evt) {
                return self.sender.send(ChannelItem::Span(encoded));
            }
        }

        if self.emit_logs {
            self.sender
                .send(ChannelItem::LogRecord(logs::encode_event(evt)));
        }
    }

    fn blocking_flush(&self, timeout: Duration) {
        emit_batcher::tokio::blocking_flush(&self.sender, timeout)
    }
}

struct Channel<T> {
    logs: Vec<T>,
    traces: Vec<T>,
}

enum ChannelItem<T> {
    LogRecord(T),
    Span(T),
}

impl<T> emit_batcher::Channel for Channel<T> {
    type Item = ChannelItem<T>;

    fn new() -> Self {
        Channel {
            logs: Vec::new(),
            traces: Vec::new(),
        }
    }

    fn push(&mut self, item: Self::Item) {
        match item {
            ChannelItem::LogRecord(item) => self.logs.push(item),
            ChannelItem::Span(item) => self.traces.push(item),
        }
    }

    fn len(&self) -> usize {
        self.logs.len() + self.traces.len()
    }

    fn clear(&mut self) {
        self.logs.clear();
        self.traces.clear();
    }
}

pub struct OtlpClientBuilder {
    resource: Option<PreEncoded>,
    scope: Option<PreEncoded>,
    encoding: Encoding,
    logs: Option<Transport>,
    traces: Option<Transport>,
}

enum Encoding {
    Proto,
}

enum Transport {
    Http { url: String },
}

impl OtlpClientBuilder {
    pub fn proto() -> Self {
        OtlpClientBuilder {
            encoding: Encoding::Proto,
            resource: None,
            scope: None,
            logs: None,
            traces: None,
        }
    }

    pub fn logs_http(mut self, dst: impl Into<String>) -> Self {
        self.logs = Some(Transport::Http { url: dst.into() });

        self
    }

    pub fn traces_http(mut self, dst: impl Into<String>) -> Self {
        self.traces = Some(Transport::Http { url: dst.into() });

        self
    }

    pub fn resource(mut self, attributes: impl emit_core::props::Props) -> Self {
        match self.encoding {
            Encoding::Proto => {
                let protobuf = sval_protobuf::stream_to_protobuf(data::Resource {
                    attributes: &data::PropsResourceAttributes(attributes),
                    dropped_attribute_count: 0,
                });

                self.resource = Some(PreEncoded::Proto(protobuf));
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
        match self.encoding {
            Encoding::Proto => {
                let protobuf = sval_protobuf::stream_to_protobuf(data::InstrumentationScope {
                    name,
                    version,
                    attributes: &data::PropsInstrumentationScopeAttributes(attributes),
                    dropped_attribute_count: 0,
                });

                self.scope = Some(PreEncoded::Proto(protobuf));
            }
        }

        self
    }

    pub fn spawn(self) -> Result<OtlpClient, Error> {
        let (sender, receiver) = emit_batcher::bounded(10_000);

        let client = OtlpSender {
            logs: match self.logs {
                Some(Transport::Http { url }) => Some(Arc::new(RawClient::Http {
                    http: HttpConnection::new(&url)?,
                    resource: self.resource.clone(),
                    scope: self.scope.clone(),
                })),
                None => None,
            },
            traces: match self.traces {
                Some(Transport::Http { url }) => Some(Arc::new(RawClient::Http {
                    http: HttpConnection::new(&url)?,
                    resource: self.resource.clone(),
                    scope: self.scope.clone(),
                })),
                None => None,
            },
        };

        let emit_logs = client.logs.is_some();
        let emit_traces = client.traces.is_some();

        emit_batcher::tokio::spawn(receiver, move |batch: Channel<PreEncoded>| {
            let client = client.clone();

            async move {
                let Channel { logs, traces } = batch;

                let mut r = Ok(());

                if let Some(client) = client.logs {
                    if let Err(e) = client.send(logs, logs::encode_request).await {
                        r = Err(e.map(|logs| Channel {
                            logs,
                            traces: Vec::new(),
                        }));
                    }
                }

                if let Some(client) = client.traces {
                    if let Err(e) = client.send(traces, traces::encode_request).await {
                        r = if let Err(re) = r {
                            Err(re.map(|mut channel| {
                                channel.traces = e.into_retryable();
                                channel
                            }))
                        } else {
                            Err(e.map(|traces| Channel {
                                traces,
                                logs: Vec::new(),
                            }))
                        };
                    }
                }

                r
            }
        });

        Ok(OtlpClient {
            emit_logs,
            emit_traces,
            sender,
        })
    }
}

#[derive(Clone)]
pub struct OtlpSender {
    // TODO: Share the client
    logs: Option<Arc<RawClient>>,
    traces: Option<Arc<RawClient>>,
}

enum RawClient {
    Http {
        http: HttpConnection,
        resource: Option<PreEncoded>,
        scope: Option<PreEncoded>,
    },
}

impl RawClient {
    pub(crate) async fn send(
        &self,
        batch: Vec<PreEncoded>,
        encode: impl FnOnce(
            Option<&PreEncoded>,
            Option<&PreEncoded>,
            &[PreEncoded],
        ) -> Result<PreEncoded, BatchError<Vec<PreEncoded>>>,
    ) -> Result<(), BatchError<Vec<PreEncoded>>> {
        match self {
            RawClient::Http {
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
