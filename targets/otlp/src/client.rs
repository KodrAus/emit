use emit_batcher::BatchError;
use std::{collections::HashMap, mem, sync::Arc, time::Duration};

use crate::{
    data::{
        self,
        logs::{LogsEventEncoder, LogsRequestEncoder},
        metrics::{MetricsEventEncoder, MetricsRequestEncoder},
        traces::{TracesEventEncoder, TracesRequestEncoder},
        EncodedEvent, EncodedPayload, EncodedScopeItems, RawEncoder, RequestEncoder,
    },
    internal_metrics::InternalMetrics,
    Error,
};

use self::http::HttpConnection;

mod http;
mod logs;
mod metrics;
mod traces;

pub use self::{logs::*, metrics::*, traces::*};

pub struct Otlp {
    otlp_logs: Option<ClientEventEncoder<LogsEventEncoder>>,
    otlp_traces: Option<ClientEventEncoder<TracesEventEncoder>>,
    otlp_metrics: Option<ClientEventEncoder<MetricsEventEncoder>>,
    metrics: Arc<InternalMetrics>,
    sender: emit_batcher::Sender<Channel>,
}

pub struct OtlpMetrics {
    channel_metrics: emit_batcher::ChannelMetrics<Channel>,
    metrics: Arc<InternalMetrics>,
}

impl emit::metric::Source for OtlpMetrics {
    fn sample_metrics<S: emit::metric::sampler::Sampler>(&self, sampler: S) {
        self.channel_metrics
            .sample_metrics(emit::metric::sampler::from_fn(|metric| {
                sampler.metric(metric.by_ref().with_module(env!("CARGO_PKG_NAME")));
            }));

        for metric in self.metrics.sample() {
            sampler.metric(metric);
        }
    }
}

impl emit::emitter::Emitter for Otlp {
    fn emit<E: emit::event::ToEvent>(&self, evt: E) {
        let evt = evt.to_event();

        if let Some(ref encoder) = self.otlp_metrics {
            if let Some(encoded) = encoder.encode_event(&evt) {
                return self.sender.send(ChannelItem::Metric(encoded));
            }
        }

        if let Some(ref encoder) = self.otlp_traces {
            if let Some(encoded) = encoder.encode_event(&evt) {
                return self.sender.send(ChannelItem::Span(encoded));
            }
        }

        if let Some(ref encoder) = self.otlp_logs {
            if let Some(encoded) = encoder.encode_event(&evt) {
                return self.sender.send(ChannelItem::LogRecord(encoded));
            }
        }

        self.metrics.otlp_event_discarded.increment();
    }

    fn blocking_flush(&self, timeout: Duration) -> bool {
        emit_batcher::tokio::blocking_flush(&self.sender, timeout)
    }
}

impl Otlp {
    pub fn metric_source(&self) -> OtlpMetrics {
        OtlpMetrics {
            channel_metrics: self.sender.metric_source(),
            metrics: self.metrics.clone(),
        }
    }
}

#[derive(Default)]
struct Channel {
    otlp_logs: EncodedScopeItems,
    otlp_traces: EncodedScopeItems,
    otlp_metrics: EncodedScopeItems,
}

enum ChannelItem {
    LogRecord(EncodedEvent),
    Span(EncodedEvent),
    Metric(EncodedEvent),
}

impl emit_batcher::Channel for Channel {
    type Item = ChannelItem;

    fn new() -> Self {
        Channel {
            otlp_logs: EncodedScopeItems::new(),
            otlp_traces: EncodedScopeItems::new(),
            otlp_metrics: EncodedScopeItems::new(),
        }
    }

    fn push(&mut self, item: Self::Item) {
        match item {
            ChannelItem::LogRecord(item) => self.otlp_logs.push(item),
            ChannelItem::Span(item) => self.otlp_traces.push(item),
            ChannelItem::Metric(item) => self.otlp_metrics.push(item),
        }
    }

    fn remaining(&self) -> usize {
        let Channel {
            otlp_logs: logs,
            otlp_traces: traces,
            otlp_metrics: metrics,
        } = self;

        logs.total_items() + traces.total_items() + metrics.total_items()
    }

    fn clear(&mut self) {
        let Channel {
            otlp_logs: logs,
            otlp_traces: traces,
            otlp_metrics: metrics,
        } = self;

        logs.clear();
        traces.clear();
        metrics.clear();
    }
}

#[must_use = "call `.spawn()` to complete the builder"]
pub struct OtlpBuilder {
    resource: Option<Resource>,
    otlp_logs: Option<OtlpLogsBuilder>,
    otlp_traces: Option<OtlpTracesBuilder>,
    otlp_metrics: Option<OtlpMetricsBuilder>,
}

struct Resource {
    attributes: HashMap<emit::Str<'static>, emit::value::OwnedValue>,
}

enum Protocol {
    Http,
    Grpc,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Encoding {
    Proto,
    Json,
}

impl Encoding {
    pub fn of(buf: &EncodedPayload) -> Self {
        match buf {
            EncodedPayload::Proto(_) => Encoding::Proto,
            EncodedPayload::Json(_) => Encoding::Json,
        }
    }
}

pub struct OtlpTransportBuilder {
    protocol: Protocol,
    url_base: String,
    url_path: Option<&'static str>,
    headers: Vec<(String, String)>,
}

impl OtlpTransportBuilder {
    pub fn http(dst: impl Into<String>) -> Self {
        OtlpTransportBuilder {
            protocol: Protocol::Http,
            url_base: dst.into(),
            url_path: None,
            headers: Vec::new(),
        }
    }

    pub fn grpc(dst: impl Into<String>) -> Self {
        OtlpTransportBuilder {
            protocol: Protocol::Grpc,
            url_base: dst.into(),
            url_path: None,
            headers: Vec::new(),
        }
    }

    pub fn headers<K: Into<String>, V: Into<String>>(
        mut self,
        headers: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        self.headers = headers
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();

        self
    }

    fn build<R>(
        self,
        metrics: Arc<InternalMetrics>,
        resource: Option<EncodedPayload>,
        request_encoder: ClientRequestEncoder<R>,
    ) -> Result<OtlpTransport<R>, Error> {
        let mut url = self.url_base;

        if let Some(path) = self.url_path {
            if !url.ends_with("/") && !path.starts_with("/") {
                url.push('/');
            }

            url.push_str(&path);
        }

        Ok(match self.protocol {
            Protocol::Http => OtlpTransport::Http {
                http: HttpConnection::http1(
                    metrics,
                    url,
                    self.headers,
                    |req| Ok(req),
                    |res| async move {
                        let status = res.http_status();

                        if status >= 200 && status < 300 {
                            Ok(vec![])
                        } else {
                            Err(Error::msg(format_args!(
                                "OTLP HTTP server responded {status}"
                            )))
                        }
                    },
                )?,
                resource,
                request_encoder,
            },
            Protocol::Grpc => OtlpTransport::Http {
                http: HttpConnection::http2(
                    metrics,
                    url,
                    self.headers,
                    |mut req| {
                        let content_type_header = match req.content_type_header() {
                            "application/x-protobuf" => "application/grpc+proto",
                            content_type => {
                                return Err(Error::msg(format_args!(
                                    "unsupported content type '{content_type}'"
                                )))
                            }
                        };

                        let len = (u32::try_from(req.content_payload_len()).unwrap()).to_be_bytes();

                        Ok(
                            if let Some(compression) = req.take_content_encoding_header() {
                                req.with_content_type_header(content_type_header)
                                    .with_headers(match compression {
                                        "gzip" => &[("grpc-encoding", "gzip")],
                                        compression => {
                                            return Err(Error::msg(format_args!(
                                                "unsupported compression '{compression}'"
                                            )))
                                        }
                                    })
                                    .with_content_frame([1, len[0], len[1], len[2], len[3]])
                            } else {
                                req.with_content_type_header(content_type_header)
                                    .with_content_frame([0, len[0], len[1], len[2], len[3]])
                            },
                        )
                    },
                    |res| async move {
                        let mut status = 0;
                        let mut msg = String::new();

                        res.stream_payload(
                            |_| {},
                            |k, v| match k {
                                "grpc-status" => {
                                    status = v.parse().unwrap_or(0);
                                }
                                "grpc-message" => {
                                    msg = v.into();
                                }
                                _ => {}
                            },
                        )
                        .await?;

                        if status == 0 {
                            Ok(vec![])
                        } else {
                            if msg.len() > 0 {
                                Err(Error::msg(format_args!(
                                    "OTLP gRPC server responded {status} {msg}"
                                )))
                            } else {
                                Err(Error::msg(format_args!(
                                    "OTLP gRPC server responded {status}"
                                )))
                            }
                        }
                    },
                )?,
                resource,
                request_encoder,
            },
        })
    }
}

impl OtlpBuilder {
    pub fn new() -> Self {
        OtlpBuilder {
            resource: None,
            otlp_logs: None,
            otlp_traces: None,
            otlp_metrics: None,
        }
    }

    pub fn logs(mut self, builder: OtlpLogsBuilder) -> Self {
        self.otlp_logs = Some(builder);
        self
    }

    pub fn traces(mut self, builder: OtlpTracesBuilder) -> Self {
        self.otlp_traces = Some(builder);
        self
    }

    pub fn metrics(mut self, builder: OtlpMetricsBuilder) -> Self {
        self.otlp_metrics = Some(builder);
        self
    }

    pub fn resource(mut self, attributes: impl emit::props::Props) -> Self {
        let mut resource = Resource {
            attributes: HashMap::new(),
        };

        attributes.for_each(|k, v| {
            resource.attributes.insert(k.to_owned(), v.to_owned());

            std::ops::ControlFlow::Continue(())
        });

        self.resource = Some(resource);

        self
    }

    pub fn spawn(self) -> Result<Otlp, Error> {
        let metrics = Arc::new(InternalMetrics::default());

        let (sender, receiver) = emit_batcher::bounded(10_000);

        let mut logs_event_encoder = None;
        let mut traces_event_encoder = None;
        let mut metrics_event_encoder = None;

        let client = OtlpClient {
            logs: match self.otlp_logs {
                Some(otlp_logs) => {
                    let (encoder, transport) =
                        otlp_logs.build(metrics.clone(), self.resource.as_ref())?;

                    logs_event_encoder = Some(encoder);
                    Some(Arc::new(transport))
                }
                None => None,
            },
            traces: match self.otlp_traces {
                Some(otlp_traces) => {
                    let (encoder, transport) =
                        otlp_traces.build(metrics.clone(), self.resource.as_ref())?;

                    traces_event_encoder = Some(encoder);
                    Some(Arc::new(transport))
                }
                None => None,
            },
            metrics: match self.otlp_metrics {
                Some(otlp_metrics) => {
                    let (encoder, transport) =
                        otlp_metrics.build(metrics.clone(), self.resource.as_ref())?;

                    metrics_event_encoder = Some(encoder);
                    Some(Arc::new(transport))
                }
                None => None,
            },
        };

        emit_batcher::tokio::spawn(receiver, move |batch: Channel| {
            let client = client.clone();

            /*
            NOTE: Possible degenerate behavior here where one signal blocks others;
            the logs endpoint is flaky and fails a lot, so it means traces also get
            backed up waiting for retries of logs to succeed.
            */
            async move {
                let Channel {
                    otlp_logs,
                    otlp_traces,
                    otlp_metrics,
                } = batch;

                let mut r = Ok::<(), BatchError<Channel>>(());

                if otlp_logs.total_scopes() > 0 {
                    if let Some(client) = client.logs {
                        send_channel_batch(&mut r, &client, otlp_logs, |channel, otlp_logs| {
                            channel.otlp_logs = otlp_logs;
                        })
                        .await;
                    }
                }

                if otlp_traces.total_scopes() > 0 {
                    if let Some(client) = client.traces {
                        send_channel_batch(&mut r, &client, otlp_traces, |channel, otlp_traces| {
                            channel.otlp_traces = otlp_traces;
                        })
                        .await;
                    }
                }

                if otlp_metrics.total_scopes() > 0 {
                    if let Some(client) = client.metrics {
                        send_channel_batch(
                            &mut r,
                            &client,
                            otlp_metrics,
                            |channel, otlp_metrics| {
                                channel.otlp_metrics = otlp_metrics;
                            },
                        )
                        .await;
                    }
                }

                r
            }
        });

        Ok(Otlp {
            otlp_logs: logs_event_encoder,
            otlp_traces: traces_event_encoder,
            otlp_metrics: metrics_event_encoder,
            metrics,
            sender,
        })
    }
}

struct ClientEventEncoder<E> {
    encoding: Encoding,
    encoder: E,
}

impl<E> ClientEventEncoder<E> {
    pub fn new(encoding: Encoding, encoder: E) -> Self {
        ClientEventEncoder { encoding, encoder }
    }
}

impl<E: data::EventEncoder> ClientEventEncoder<E> {
    pub fn encode_event(
        &self,
        evt: &emit::event::Event<impl emit::props::Props>,
    ) -> Option<EncodedEvent> {
        match self.encoding {
            Encoding::Proto => self.encoder.encode_event::<data::Proto>(evt),
            Encoding::Json => self.encoder.encode_event::<data::Json>(evt),
        }
    }
}

struct ClientRequestEncoder<R> {
    encoding: Encoding,
    encoder: R,
}

impl<R> ClientRequestEncoder<R> {
    pub fn new(encoding: Encoding, encoder: R) -> Self {
        ClientRequestEncoder { encoding, encoder }
    }
}

impl<R: data::RequestEncoder> ClientRequestEncoder<R> {
    pub fn encode_request(
        &self,
        resource: Option<&EncodedPayload>,
        items: &EncodedScopeItems,
    ) -> Result<EncodedPayload, BatchError<EncodedScopeItems>> {
        match self.encoding {
            Encoding::Proto => self
                .encoder
                .encode_request::<data::Proto>(resource, items)
                .map_err(BatchError::no_retry),
            Encoding::Json => self
                .encoder
                .encode_request::<data::Json>(resource, items)
                .map_err(BatchError::no_retry),
        }
    }
}

fn encode_resource(encoding: Encoding, resource: &Resource) -> EncodedPayload {
    let attributes = data::PropsResourceAttributes(&resource.attributes);

    let resource = data::Resource {
        attributes: &attributes,
    };

    match encoding {
        Encoding::Proto => data::Proto::encode(&resource),
        Encoding::Json => data::Json::encode(&resource),
    }
}

#[derive(Clone)]
pub struct OtlpClient {
    logs: Option<Arc<OtlpTransport<LogsRequestEncoder>>>,
    traces: Option<Arc<OtlpTransport<TracesRequestEncoder>>>,
    metrics: Option<Arc<OtlpTransport<MetricsRequestEncoder>>>,
}

enum OtlpTransport<R> {
    Http {
        http: HttpConnection,
        resource: Option<EncodedPayload>,
        request_encoder: ClientRequestEncoder<R>,
    },
}

impl<R: data::RequestEncoder> OtlpTransport<R> {
    #[emit::span(rt: emit::runtime::internal(), arg: span, "send OTLP batch of {batch_size} events", batch_size: batch.total_scopes())]
    pub(crate) async fn send(
        &self,
        batch: EncodedScopeItems,
    ) -> Result<(), BatchError<EncodedScopeItems>> {
        match self {
            OtlpTransport::Http {
                ref http,
                ref resource,
                ref request_encoder,
            } => {
                let uri = http.uri();
                let batch_size = batch.total_items();

                match http
                    .send(request_encoder.encode_request(resource.as_ref(), &batch)?)
                    .await
                {
                    Ok(res) => {
                        span.complete_with(|span| {
                            emit::debug!(
                                rt: emit::runtime::internal(),
                                extent: span.extent(),
                                props: span.props(),
                                "OTLP batch of {batch_size} events to {uri}",
                                batch_size,
                            )
                        });

                        res
                    }
                    Err(err) => {
                        span.complete_with(|span| {
                            emit::warn!(
                                rt: emit::runtime::internal(),
                                extent: span.extent(),
                                props: span.props(),
                                "OTLP batch of {batch_size} events to {uri} failed: {err}",
                                batch_size,
                                err,
                            )
                        });

                        return Err(BatchError::retry(err, batch));
                    }
                };
            }
        }

        Ok(())
    }
}

async fn send_channel_batch<R: RequestEncoder>(
    r: &mut Result<(), BatchError<Channel>>,
    client: &OtlpTransport<R>,
    batch: EncodedScopeItems,
    set: impl Fn(&mut Channel, EncodedScopeItems),
) {
    // Attempt to send the batch, restoring it on the channel if it fails
    if let Err(e) = client.send(batch).await {
        *r = if let Err(re) = mem::replace(r, Ok(())) {
            Err(re.map_retryable(|channel| {
                let mut channel = channel.unwrap_or_default();
                set(&mut channel, e.into_retryable().unwrap_or_default());

                Some(channel)
            }))
        } else {
            Err(e.map_retryable(|batch| {
                let mut channel = Channel::default();
                set(&mut channel, batch.unwrap_or_default());

                Some(channel)
            }))
        };
    }
}
