use emit_batcher::BatchError;
use std::{collections::HashMap, fmt, sync::Arc, time::Duration};

use crate::{
    data::{self, logs, metrics, traces, PreEncoded, RawEncoder},
    internal_metrics::InternalMetrics,
    Error,
};

use self::http::HttpConnection;

mod http;

pub struct Otlp {
    otlp_logs: Option<ClientEventEncoder<logs::LogsEventEncoder>>,
    otlp_traces: Option<ClientEventEncoder<traces::TracesEventEncoder>>,
    otlp_metrics: Option<ClientEventEncoder<metrics::MetricsEventEncoder>>,
    metrics: Arc<InternalMetrics>,
    sender: emit_batcher::Sender<Channel<PreEncoded>>,
}

impl Otlp {
    pub fn sample_metrics<'a>(
        &'a self,
    ) -> impl Iterator<Item = emit::metric::Metric<'static, emit::empty::Empty>> + 'a {
        self.sender
            .sample_metrics()
            .map(|metric| metric.with_module("emit_otlp"))
            .chain(self.metrics.sample())
    }
}

impl emit::emitter::Emitter for Otlp {
    fn emit<P: emit::props::Props>(&self, evt: &emit::event::Event<P>) {
        if let Some(ref encoder) = self.otlp_metrics {
            if let Some(encoded) = encoder.encode_event(evt) {
                return self.sender.send(ChannelItem::Metric(encoded));
            }
        }

        if let Some(ref encoder) = self.otlp_traces {
            if let Some(encoded) = encoder.encode_event(evt) {
                return self.sender.send(ChannelItem::Span(encoded));
            }
        }

        if let Some(ref encoder) = self.otlp_logs {
            if let Some(encoded) = encoder.encode_event(evt) {
                return self.sender.send(ChannelItem::LogRecord(encoded));
            }
        }

        self.metrics.otlp_event_discarded.increment();
    }

    fn blocking_flush(&self, timeout: Duration) {
        emit_batcher::tokio::blocking_flush(&self.sender, timeout);

        let rt = emit::runtime::internal_slot();
        if rt.is_enabled() {
            let rt = rt.get();

            for metric in self.sample_metrics() {
                emit::emit!(
                    rt,
                    extent: metric.extent(),
                    props: metric.props(),
                    "{metric_agg} of {metric_name} is {metric_value}",
                    metric_name: metric.name(),
                    metric_agg: metric.agg(),
                    metric_value: metric.value(),
                );
            }
        }
    }
}

struct Channel<T> {
    otlp_logs: Vec<T>,
    otlp_traces: Vec<T>,
    otlp_metrics: Vec<T>,
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
            otlp_logs: Vec::new(),
            otlp_traces: Vec::new(),
            otlp_metrics: Vec::new(),
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

        logs.len() + traces.len() + metrics.len()
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

pub struct OtlpBuilder {
    resource: Option<Resource>,
    scope: Option<Scope>,
    otlp_logs: Option<OtlpLogsBuilder>,
    otlp_traces: Option<OtlpTracesBuilder>,
    otlp_metrics: Option<OtlpMetricsBuilder>,
}

struct Resource {
    attributes: HashMap<emit::Str<'static>, emit::value::OwnedValue>,
}

struct Scope {
    name: String,
    version: String,
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
    pub fn of(buf: &PreEncoded) -> Self {
        match buf {
            PreEncoded::Proto(_) => Encoding::Proto,
            PreEncoded::Json(_) => Encoding::Json,
        }
    }
}

pub struct OtlpLogsBuilder {
    event_encoder: logs::LogsEventEncoder,
    request_encoder: logs::LogsRequestEncoder,
    encoding: Encoding,
    transport: OtlpTransportBuilder,
}

impl OtlpLogsBuilder {
    fn new(encoding: Encoding, transport: OtlpTransportBuilder) -> Self {
        OtlpLogsBuilder {
            event_encoder: logs::LogsEventEncoder::default(),
            request_encoder: logs::LogsRequestEncoder::default(),
            encoding,
            transport,
        }
    }

    pub fn proto(mut transport: OtlpTransportBuilder) -> Self {
        if let Protocol::Grpc = transport.protocol {
            transport.url_path = Some("opentelemetry.proto.collector.logs.v1.LogsService/Export");
        }

        Self::new(Encoding::Proto, transport)
    }

    pub fn http_proto(dst: impl Into<String>) -> Self {
        Self::proto(OtlpTransportBuilder::http(dst))
    }

    pub fn grpc_proto(dst: impl Into<String>) -> Self {
        Self::proto(OtlpTransportBuilder::grpc(dst))
    }

    pub fn json(transport: OtlpTransportBuilder) -> Self {
        Self::new(Encoding::Json, transport)
    }

    pub fn http_json(dst: impl Into<String>) -> Self {
        Self::json(OtlpTransportBuilder::http(dst))
    }

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
        self.event_encoder.body = Box::new(writer);
        self
    }
}

pub struct OtlpTracesBuilder {
    event_encoder: traces::TracesEventEncoder,
    request_encoder: traces::TracesRequestEncoder,
    encoding: Encoding,
    transport: OtlpTransportBuilder,
}

impl OtlpTracesBuilder {
    fn new(encoding: Encoding, transport: OtlpTransportBuilder) -> Self {
        OtlpTracesBuilder {
            event_encoder: traces::TracesEventEncoder::default(),
            request_encoder: traces::TracesRequestEncoder::default(),
            encoding,
            transport,
        }
    }

    pub fn proto(mut transport: OtlpTransportBuilder) -> Self {
        if let Protocol::Grpc = transport.protocol {
            transport.url_path = Some("opentelemetry.proto.collector.trace.v1.TraceService/Export");
        }

        Self::new(Encoding::Proto, transport)
    }

    pub fn http_proto(dst: impl Into<String>) -> Self {
        Self::proto(OtlpTransportBuilder::http(dst))
    }

    pub fn grpc_proto(dst: impl Into<String>) -> Self {
        Self::proto(OtlpTransportBuilder::grpc(dst))
    }

    pub fn json(transport: OtlpTransportBuilder) -> Self {
        Self::new(Encoding::Json, transport)
    }

    pub fn http_json(dst: impl Into<String>) -> Self {
        Self::json(OtlpTransportBuilder::http(dst))
    }

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
        self.event_encoder.name = Box::new(writer);
        self
    }
}

pub struct OtlpMetricsBuilder {
    event_encoder: metrics::MetricsEventEncoder,
    request_encoder: metrics::MetricsRequestEncoder,
    encoding: Encoding,
    transport: OtlpTransportBuilder,
}

impl OtlpMetricsBuilder {
    fn new(encoding: Encoding, transport: OtlpTransportBuilder) -> Self {
        OtlpMetricsBuilder {
            event_encoder: metrics::MetricsEventEncoder::default(),
            request_encoder: metrics::MetricsRequestEncoder::default(),
            encoding,
            transport,
        }
    }

    pub fn proto(mut transport: OtlpTransportBuilder) -> Self {
        if let Protocol::Grpc = transport.protocol {
            transport.url_path =
                Some("opentelemetry.proto.collector.metrics.v1.MetricsService/Export");
        }

        Self::new(Encoding::Proto, transport)
    }

    pub fn http_proto(dst: impl Into<String>) -> Self {
        Self::proto(OtlpTransportBuilder::http(dst))
    }

    pub fn grpc_proto(dst: impl Into<String>) -> Self {
        Self::proto(OtlpTransportBuilder::grpc(dst))
    }

    pub fn json(transport: OtlpTransportBuilder) -> Self {
        Self::new(Encoding::Json, transport)
    }

    pub fn http_json(dst: impl Into<String>) -> Self {
        Self::json(OtlpTransportBuilder::http(dst))
    }

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
        self.event_encoder.name = Box::new(writer);
        self
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
        resource: Option<PreEncoded>,
        scope: Option<PreEncoded>,
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
                scope,
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
                scope,
                request_encoder,
            },
        })
    }
}

impl OtlpBuilder {
    pub fn new() -> Self {
        OtlpBuilder {
            resource: None,
            scope: None,
            otlp_logs: None,
            otlp_traces: None,
            otlp_metrics: None,
        }
    }

    pub fn logs_http_proto(self, dst: impl Into<String>) -> Self {
        self.logs(OtlpLogsBuilder::http_proto(dst))
    }

    pub fn logs_grpc_proto(self, dst: impl Into<String>) -> Self {
        self.logs(OtlpLogsBuilder::grpc_proto(dst))
    }

    pub fn logs_http_json(self, dst: impl Into<String>) -> Self {
        self.logs(OtlpLogsBuilder::http_json(dst))
    }

    pub fn logs(mut self, builder: OtlpLogsBuilder) -> Self {
        self.otlp_logs = Some(builder);
        self
    }

    pub fn traces_http_proto(self, dst: impl Into<String>) -> Self {
        self.traces(OtlpTracesBuilder::http_proto(dst))
    }

    pub fn traces_grpc_proto(self, dst: impl Into<String>) -> Self {
        self.traces(OtlpTracesBuilder::grpc_proto(dst))
    }

    pub fn traces_http_json(self, dst: impl Into<String>) -> Self {
        self.traces(OtlpTracesBuilder::http_json(dst))
    }

    pub fn traces(mut self, builder: OtlpTracesBuilder) -> Self {
        self.otlp_traces = Some(builder);
        self
    }

    pub fn metrics_http_proto(self, dst: impl Into<String>) -> Self {
        self.metrics(OtlpMetricsBuilder::http_proto(dst))
    }

    pub fn metrics_grpc_proto(self, dst: impl Into<String>) -> Self {
        self.metrics(OtlpMetricsBuilder::grpc_proto(dst))
    }

    pub fn metrics_http_json(self, dst: impl Into<String>) -> Self {
        self.metrics(OtlpMetricsBuilder::http_json(dst))
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

    pub fn scope(
        mut self,
        name: impl Into<String>,
        version: impl Into<String>,
        attributes: impl emit::props::Props,
    ) -> Self {
        let mut scope = Scope {
            name: name.into(),
            version: version.into(),
            attributes: HashMap::new(),
        };

        attributes.for_each(|k, v| {
            scope.attributes.insert(k.to_owned(), v.to_owned());

            std::ops::ControlFlow::Continue(())
        });

        self.scope = Some(scope);

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
                Some(OtlpLogsBuilder {
                    event_encoder,
                    request_encoder,
                    encoding,
                    transport,
                }) => {
                    logs_event_encoder = Some(ClientEventEncoder::new(encoding, event_encoder));

                    Some(Arc::new(
                        transport.build(
                            metrics.clone(),
                            self.resource
                                .as_ref()
                                .map(|resource| encode_resource(encoding, resource)),
                            self.scope
                                .as_ref()
                                .map(|scope| encode_scope(encoding, scope)),
                            ClientRequestEncoder::new(encoding, request_encoder),
                        )?,
                    ))
                }
                None => None,
            },
            traces: match self.otlp_traces {
                Some(OtlpTracesBuilder {
                    event_encoder,
                    request_encoder,
                    encoding,
                    transport,
                }) => {
                    traces_event_encoder = Some(ClientEventEncoder::new(encoding, event_encoder));

                    Some(Arc::new(
                        transport.build(
                            metrics.clone(),
                            self.resource
                                .as_ref()
                                .map(|resource| encode_resource(encoding, resource)),
                            self.scope
                                .as_ref()
                                .map(|scope| encode_scope(encoding, scope)),
                            ClientRequestEncoder::new(encoding, request_encoder),
                        )?,
                    ))
                }
                None => None,
            },
            metrics: match self.otlp_metrics {
                Some(OtlpMetricsBuilder {
                    event_encoder,
                    request_encoder,
                    encoding,
                    transport,
                }) => {
                    metrics_event_encoder = Some(ClientEventEncoder::new(encoding, event_encoder));

                    Some(Arc::new(
                        transport.build(
                            metrics.clone(),
                            self.resource
                                .as_ref()
                                .map(|resource| encode_resource(encoding, resource)),
                            self.scope
                                .as_ref()
                                .map(|scope| encode_scope(encoding, scope)),
                            ClientRequestEncoder::new(encoding, request_encoder),
                        )?,
                    ))
                }
                None => None,
            },
        };

        emit_batcher::tokio::spawn(receiver, move |batch: Channel<PreEncoded>| {
            let client = client.clone();

            async move {
                let Channel {
                    otlp_logs,
                    otlp_traces,
                    otlp_metrics,
                } = batch;

                let mut r = Ok(());

                if otlp_logs.len() > 0 {
                    if let Some(client) = client.logs {
                        if let Err(e) = client.send(otlp_logs).await {
                            r = Err(e.map(|logs| Channel {
                                otlp_logs: logs,
                                otlp_traces: Vec::new(),
                                otlp_metrics: Vec::new(),
                            }));
                        }
                    }
                }

                if otlp_traces.len() > 0 {
                    if let Some(client) = client.traces {
                        if let Err(e) = client.send(otlp_traces).await {
                            r = if let Err(re) = r {
                                Err(re.map(|mut channel| {
                                    channel.otlp_traces = e.into_retryable();
                                    channel
                                }))
                            } else {
                                Err(e.map(|traces| Channel {
                                    otlp_traces: traces,
                                    otlp_logs: Vec::new(),
                                    otlp_metrics: Vec::new(),
                                }))
                            };
                        }
                    }
                }

                if otlp_metrics.len() > 0 {
                    if let Some(client) = client.metrics {
                        if let Err(e) = client.send(otlp_metrics).await {
                            r = if let Err(re) = r {
                                Err(re.map(|mut channel| {
                                    channel.otlp_metrics = e.into_retryable();
                                    channel
                                }))
                            } else {
                                Err(e.map(|metrics| Channel {
                                    otlp_metrics: metrics,
                                    otlp_logs: Vec::new(),
                                    otlp_traces: Vec::new(),
                                }))
                            };
                        }
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
    ) -> Option<PreEncoded> {
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
        resource: Option<&PreEncoded>,
        scope: Option<&PreEncoded>,
        items: &[PreEncoded],
    ) -> Result<PreEncoded, BatchError<Vec<PreEncoded>>> {
        match self.encoding {
            Encoding::Proto => self
                .encoder
                .encode_request::<data::Proto>(resource, scope, items),
            Encoding::Json => self
                .encoder
                .encode_request::<data::Json>(resource, scope, items),
        }
    }
}

fn encode_resource(encoding: Encoding, resource: &Resource) -> PreEncoded {
    let attributes = data::PropsResourceAttributes(&resource.attributes);

    let resource = data::Resource {
        attributes: &attributes,
        dropped_attribute_count: 0,
    };

    match encoding {
        Encoding::Proto => data::Proto::encode(&resource),
        Encoding::Json => data::Json::encode(&resource),
    }
}

fn encode_scope(encoding: Encoding, scope: &Scope) -> PreEncoded {
    let attributes = data::PropsInstrumentationScopeAttributes(&scope.attributes);

    let scope = data::InstrumentationScope {
        name: &scope.name,
        version: &scope.version,
        attributes: &attributes,
        dropped_attribute_count: 0,
    };

    match encoding {
        Encoding::Proto => data::Proto::encode(&scope),
        Encoding::Json => data::Json::encode(&scope),
    }
}

#[derive(Clone)]
pub struct OtlpClient {
    logs: Option<Arc<OtlpTransport<logs::LogsRequestEncoder>>>,
    traces: Option<Arc<OtlpTransport<traces::TracesRequestEncoder>>>,
    metrics: Option<Arc<OtlpTransport<metrics::MetricsRequestEncoder>>>,
}

enum OtlpTransport<R> {
    Http {
        http: HttpConnection,
        resource: Option<PreEncoded>,
        scope: Option<PreEncoded>,
        request_encoder: ClientRequestEncoder<R>,
    },
}

impl<R: data::RequestEncoder> OtlpTransport<R> {
    #[emit::span(rt: emit::runtime::internal(), arg: span, "send")]
    pub(crate) async fn send(
        &self,
        batch: Vec<PreEncoded>,
    ) -> Result<(), BatchError<Vec<PreEncoded>>> {
        match self {
            OtlpTransport::Http {
                ref http,
                ref resource,
                ref scope,
                ref request_encoder,
            } => {
                let uri = http.uri();
                let batch_size = batch.len();

                match http
                    .send(request_encoder.encode_request(
                        resource.as_ref(),
                        scope.as_ref(),
                        &batch,
                    )?)
                    .await
                {
                    Ok(res) => {
                        span.complete(|extent| {
                            emit::debug!(
                                rt: emit::runtime::internal(),
                                when: emit::filter::always(),
                                extent,
                                "OTLP batch of {batch_size} events to {uri}",
                                batch_size,
                            )
                        });

                        res
                    }
                    Err(err) => {
                        span.complete(|extent| {
                            emit::warn!(
                                rt: emit::runtime::internal(),
                                when: emit::filter::always(),
                                extent,
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
