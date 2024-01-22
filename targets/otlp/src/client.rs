use emit_batcher::BatchError;
use std::{fmt, sync::Arc, time::Duration};

use crate::{
    data::{self, default_message_formatter, logs, metrics, traces, PreEncoded},
    Error,
};

use self::http::HttpConnection;

mod http;

pub struct OtlpClient {
    logs: Option<logs::EventEncoder>,
    traces: Option<traces::EventEncoder>,
    metrics: Option<metrics::EventEncoder>,
    sender: emit_batcher::Sender<Channel<PreEncoded>>,
}

impl emit::emitter::Emitter for OtlpClient {
    fn emit<P: emit::props::Props>(&self, evt: &emit::event::Event<P>) {
        if let Some(ref encoder) = self.metrics {
            if let Some(encoded) = encoder.encode_event(evt) {
                return self.sender.send(ChannelItem::Metric(encoded));
            }
        }

        if let Some(ref encoder) = self.traces {
            if let Some(encoded) = encoder.encode_event(evt) {
                return self.sender.send(ChannelItem::Span(encoded));
            }
        }

        if let Some(ref encoder) = self.logs {
            self.sender
                .send(ChannelItem::LogRecord(encoder.encode_event(evt)));
        }
    }

    fn blocking_flush(&self, timeout: Duration) {
        emit_batcher::tokio::blocking_flush(&self.sender, timeout)
    }
}

struct Channel<T> {
    logs: Vec<T>,
    traces: Vec<T>,
    metrics: Vec<T>,
}

enum ChannelItem<T> {
    LogRecord(T),
    Span(T),
    Metric(T),
}

impl<T> emit_batcher::Channel for Channel<T> {
    type Item = ChannelItem<T>;

    fn new() -> Self {
        Channel {
            logs: Vec::new(),
            traces: Vec::new(),
            metrics: Vec::new(),
        }
    }

    fn push(&mut self, item: Self::Item) {
        match item {
            ChannelItem::LogRecord(item) => self.logs.push(item),
            ChannelItem::Span(item) => self.traces.push(item),
            ChannelItem::Metric(item) => self.metrics.push(item),
        }
    }

    fn remaining(&self) -> usize {
        let Channel {
            logs,
            traces,
            metrics,
        } = self;

        logs.len() + traces.len() + metrics.len()
    }

    fn clear(&mut self) {
        let Channel {
            logs,
            traces,
            metrics,
        } = self;

        logs.clear();
        traces.clear();
        metrics.clear();
    }
}

pub struct OtlpClientBuilder {
    resource: Option<PreEncoded>,
    scope: Option<PreEncoded>,
    encoding: Encoding,
    logs: Option<OtlpLogsBuilder>,
    traces: Option<OtlpTracesBuilder>,
    metrics: Option<OtlpMetricsBuilder>,
}

pub struct OtlpLogsBuilder {
    encoder: logs::EventEncoder,
    transport: Transport,
}

impl OtlpLogsBuilder {
    pub fn http(dst: impl Into<String>) -> Self {
        OtlpLogsBuilder {
            encoder: logs::EventEncoder {
                body: default_message_formatter(),
            },
            transport: Transport::Http { url: dst.into() },
        }
    }
}

pub struct OtlpTracesBuilder {
    encoder: traces::EventEncoder,
    transport: Transport,
}

impl OtlpTracesBuilder {
    pub fn http(dst: impl Into<String>) -> Self {
        OtlpTracesBuilder {
            encoder: traces::EventEncoder {
                name: default_message_formatter(),
            },
            transport: Transport::Http { url: dst.into() },
        }
    }
}

pub struct OtlpMetricsBuilder {
    encoder: metrics::EventEncoder,
    transport: Transport,
}

impl OtlpMetricsBuilder {
    pub fn http(dst: impl Into<String>) -> Self {
        OtlpMetricsBuilder {
            encoder: metrics::EventEncoder {
                name: default_message_formatter(),
            },
            transport: Transport::Http { url: dst.into() },
        }
    }
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
            metrics: None,
        }
    }

    pub fn logs_http(self, dst: impl Into<String>) -> Self {
        self.logs(OtlpLogsBuilder::http(dst))
    }

    pub fn logs(mut self, builder: OtlpLogsBuilder) -> Self {
        self.logs = Some(builder);
        self
    }

    pub fn traces_http(self, dst: impl Into<String>) -> Self {
        self.traces(OtlpTracesBuilder::http(dst))
    }

    pub fn traces(mut self, builder: OtlpTracesBuilder) -> Self {
        self.traces = Some(builder);
        self
    }

    pub fn metrics_http(self, dst: impl Into<String>) -> Self {
        self.metrics(OtlpMetricsBuilder::http(dst))
    }

    pub fn metrics(mut self, builder: OtlpMetricsBuilder) -> Self {
        self.metrics = Some(builder);
        self
    }

    pub fn resource(mut self, attributes: impl emit::props::Props) -> Self {
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

    pub fn scope(mut self, name: &str, version: &str, attributes: impl emit::props::Props) -> Self {
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

        let mut logs = None;
        let mut traces = None;
        let mut metrics = None;

        let client = OtlpSender {
            logs: match self.logs {
                Some(OtlpLogsBuilder {
                    encoder,
                    transport: Transport::Http { url },
                }) => {
                    logs = Some(encoder);
                    Some(Arc::new(RawClient::Http {
                        http: HttpConnection::new(&url)?,
                        resource: self.resource.clone(),
                        scope: self.scope.clone(),
                    }))
                }
                None => None,
            },
            traces: match self.traces {
                Some(OtlpTracesBuilder {
                    encoder,
                    transport: Transport::Http { url },
                }) => {
                    traces = Some(encoder);
                    Some(Arc::new(RawClient::Http {
                        http: HttpConnection::new(&url)?,
                        resource: self.resource.clone(),
                        scope: self.scope.clone(),
                    }))
                }
                None => None,
            },
            metrics: match self.metrics {
                Some(OtlpMetricsBuilder {
                    encoder,
                    transport: Transport::Http { url },
                }) => {
                    metrics = Some(encoder);
                    Some(Arc::new(RawClient::Http {
                        http: HttpConnection::new(&url)?,
                        resource: self.resource.clone(),
                        scope: self.scope.clone(),
                    }))
                }
                None => None,
            },
        };

        emit_batcher::tokio::spawn(receiver, move |batch: Channel<PreEncoded>| {
            let client = client.clone();

            async move {
                let Channel {
                    logs,
                    traces,
                    metrics,
                } = batch;

                let mut r = Ok(());

                if logs.len() > 0 {
                    if let Some(client) = client.logs {
                        if let Err(e) = client
                            .send(logs, logs::encode_request, {
                                #[cfg(feature = "decode_responses")]
                                {
                                    if emit::runtime::internal_slot().is_enabled() {
                                        Some(logs::decode_response)
                                    } else {
                                        None
                                    }
                                }
                                #[cfg(not(feature = "decode_responses"))]
                                {
                                    None::<fn(Result<&[u8], &[u8]>)>
                                }
                            })
                            .await
                        {
                            r = Err(e.map(|logs| Channel {
                                logs,
                                traces: Vec::new(),
                                metrics: Vec::new(),
                            }));
                        }
                    }
                }

                if traces.len() > 0 {
                    if let Some(client) = client.traces {
                        if let Err(e) = client
                            .send(traces, traces::encode_request, {
                                #[cfg(feature = "decode_responses")]
                                {
                                    if emit::runtime::internal_slot().is_enabled() {
                                        Some(traces::decode_response)
                                    } else {
                                        None
                                    }
                                }
                                #[cfg(not(feature = "decode_responses"))]
                                {
                                    None::<fn(Result<&[u8], &[u8]>)>
                                }
                            })
                            .await
                        {
                            r = if let Err(re) = r {
                                Err(re.map(|mut channel| {
                                    channel.traces = e.into_retryable();
                                    channel
                                }))
                            } else {
                                Err(e.map(|traces| Channel {
                                    traces,
                                    logs: Vec::new(),
                                    metrics: Vec::new(),
                                }))
                            };
                        }
                    }
                }

                if metrics.len() > 0 {
                    if let Some(client) = client.metrics {
                        if let Err(e) = client
                            .send(metrics, metrics::encode_request, {
                                #[cfg(feature = "decode_responses")]
                                {
                                    if emit::runtime::internal_slot().is_enabled() {
                                        Some(metrics::decode_response)
                                    } else {
                                        None
                                    }
                                }
                                #[cfg(not(feature = "decode_responses"))]
                                {
                                    None::<fn(Result<&[u8], &[u8]>)>
                                }
                            })
                            .await
                        {
                            r = if let Err(re) = r {
                                Err(re.map(|mut channel| {
                                    channel.metrics = e.into_retryable();
                                    channel
                                }))
                            } else {
                                Err(e.map(|metrics| Channel {
                                    metrics,
                                    logs: Vec::new(),
                                    traces: Vec::new(),
                                }))
                            };
                        }
                    }
                }

                r
            }
        });

        Ok(OtlpClient {
            logs,
            traces,
            metrics,
            sender,
        })
    }
}

impl OtlpLogsBuilder {
    pub fn body(
        mut self,
        writer: impl Fn(
                &emit::event::Event<&dyn emit::props::ErasedProps>,
                &mut fmt::Formatter,
            ) -> fmt::Result
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.encoder.body = Box::new(writer);
        self
    }
}

impl OtlpTracesBuilder {
    pub fn name(
        mut self,
        writer: impl Fn(
                &emit::event::Event<&dyn emit::props::ErasedProps>,
                &mut fmt::Formatter,
            ) -> fmt::Result
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.encoder.name = Box::new(writer);
        self
    }
}

impl OtlpMetricsBuilder {
    pub fn name(
        mut self,
        writer: impl Fn(
                &emit::event::Event<&dyn emit::props::ErasedProps>,
                &mut fmt::Formatter,
            ) -> fmt::Result
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.encoder.name = Box::new(writer);
        self
    }
}

#[derive(Clone)]
pub struct OtlpSender {
    // TODO: Share the client
    logs: Option<Arc<RawClient>>,
    traces: Option<Arc<RawClient>>,
    metrics: Option<Arc<RawClient>>,
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
        decode: Option<impl FnOnce(Result<&[u8], &[u8]>)>,
    ) -> Result<(), BatchError<Vec<PreEncoded>>> {
        use emit::IdRng as _;

        let rt = emit::runtime::internal();

        let ctxt = emit::frame::Frame::new(
            rt.ctxt(),
            emit::props! {
                trace_id: rt.gen_trace_id(),
                span_id: rt.gen_span_id(),
            },
        );

        ctxt.with_future(async move {
            match self {
                RawClient::Http {
                    ref http,
                    ref resource,
                    ref scope,
                } => {
                    let batch_size = batch.len();

                    let timer = emit::clock::Timer::start(rt.clock());

                    let res = http
                        .send(encode(resource.as_ref(), scope.as_ref(), &batch)?)
                        .await
                        .map_err(|err| {
                            emit::warn!(rt, extent: timer, "OTLP batch of {batch_size} failed to send: {err}");

                            BatchError::retry(err, batch)
                        })?;

                    emit::debug!(rt, extent: timer, "OTLP batch of {batch_size} responded {status_code: res.status()}");

                    if let Some(decode) = decode {
                        let status = res.status();
                        let body = res
                            .read_to_vec()
                            .await
                            .map_err(|err| {
                                emit::warn!(rt, "failed to read OTLP response: {err}");

                                BatchError::no_retry(err)
                            })?;

                        if status >= 200 && status < 300 {
                            decode(Ok(&body));
                        } else {
                            decode(Err(&body));
                        }
                    }
                }
            }

            Ok(())
        }).await
    }
}
