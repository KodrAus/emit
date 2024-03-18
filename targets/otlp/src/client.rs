use emit_batcher::BatchError;
use std::{collections::HashMap, fmt, sync::Arc, time::Duration};

use crate::{
    data::{self, logs, metrics, traces, PreEncoded},
    internal_metrics::InternalMetrics,
    Error,
};

use self::http::HttpConnection;

mod http;

pub struct Otlp {
    otlp_logs: Option<logs::EventEncoder>,
    otlp_traces: Option<traces::EventEncoder>,
    otlp_metrics: Option<metrics::EventEncoder>,
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
            self.sender
                .send(ChannelItem::LogRecord(encoder.encode_event(evt)));
            return;
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

pub struct OtlpLogsBuilder {
    encoder: logs::EventEncoder,
    encoding: Encoding,
    transport: OtlpTransportBuilder,
}

impl OtlpLogsBuilder {
    pub fn proto(transport: OtlpTransportBuilder) -> Self {
        OtlpLogsBuilder {
            encoder: logs::EventEncoder::default(),
            encoding: Encoding::Proto,
            transport,
        }
    }

    pub fn http_proto(dst: impl Into<String>) -> Self {
        Self::proto(OtlpTransportBuilder::http(dst))
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
        self.encoder.body = Box::new(writer);
        self
    }
}

pub struct OtlpTracesBuilder {
    encoder: traces::EventEncoder,
    encoding: Encoding,
    transport: OtlpTransportBuilder,
}

impl OtlpTracesBuilder {
    pub fn proto(transport: OtlpTransportBuilder) -> Self {
        OtlpTracesBuilder {
            encoder: traces::EventEncoder::default(),
            encoding: Encoding::Proto,
            transport,
        }
    }

    pub fn http_proto(dst: impl Into<String>) -> Self {
        Self::proto(OtlpTransportBuilder::http(dst))
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
        self.encoder.name = Box::new(writer);
        self
    }
}

pub struct OtlpMetricsBuilder {
    encoder: metrics::EventEncoder,
    encoding: Encoding,
    transport: OtlpTransportBuilder,
}

impl OtlpMetricsBuilder {
    pub fn proto(transport: OtlpTransportBuilder) -> Self {
        OtlpMetricsBuilder {
            encoder: metrics::EventEncoder::default(),
            encoding: Encoding::Proto,
            transport,
        }
    }

    pub fn http_proto(dst: impl Into<String>) -> Self {
        Self::proto(OtlpTransportBuilder::http(dst))
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
        self.encoder.name = Box::new(writer);
        self
    }
}

enum Encoding {
    Proto,
}

enum Protocol {
    Http,
}

pub struct OtlpTransportBuilder {
    protocol: Protocol,
    url: String,
    headers: Vec<(String, String)>,
}

impl OtlpTransportBuilder {
    pub fn http(dst: impl Into<String>) -> Self {
        OtlpTransportBuilder {
            protocol: Protocol::Http,
            url: dst.into(),
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

    fn build(
        self,
        resource: Option<PreEncoded>,
        scope: Option<PreEncoded>,
    ) -> Result<OtlpTransport, Error> {
        Ok(match self.protocol {
            Protocol::Http => OtlpTransport::Http {
                http: HttpConnection::new(self.url, self.headers)?,
                resource,
                scope,
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

    pub fn logs(mut self, builder: OtlpLogsBuilder) -> Self {
        self.otlp_logs = Some(builder);
        self
    }

    pub fn traces_http_proto(self, dst: impl Into<String>) -> Self {
        self.traces(OtlpTracesBuilder::http_proto(dst))
    }

    pub fn traces(mut self, builder: OtlpTracesBuilder) -> Self {
        self.otlp_traces = Some(builder);
        self
    }

    pub fn metrics_http_proto(self, dst: impl Into<String>) -> Self {
        self.metrics(OtlpMetricsBuilder::http_proto(dst))
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
        let (sender, receiver) = emit_batcher::bounded(10_000);

        let mut otlp_logs = None;
        let mut otlp_traces = None;
        let mut otlp_metrics = None;

        let client = OtlpClient {
            otlp_logs: match self.otlp_logs {
                Some(OtlpLogsBuilder {
                    encoder,
                    encoding: Encoding::Proto,
                    transport,
                }) => {
                    otlp_logs = Some(encoder);
                    Some(Arc::new(transport.build(
                        self.resource.as_ref().map(resource_proto),
                        self.scope.as_ref().map(scope_proto),
                    )?))
                }
                None => None,
            },
            otlp_traces: match self.otlp_traces {
                Some(OtlpTracesBuilder {
                    encoder,
                    encoding: Encoding::Proto,
                    transport,
                }) => {
                    otlp_traces = Some(encoder);
                    Some(Arc::new(transport.build(
                        self.resource.as_ref().map(resource_proto),
                        self.scope.as_ref().map(scope_proto),
                    )?))
                }
                None => None,
            },
            otlp_metrics: match self.otlp_metrics {
                Some(OtlpMetricsBuilder {
                    encoder,
                    encoding: Encoding::Proto,
                    transport,
                }) => {
                    otlp_metrics = Some(encoder);
                    Some(Arc::new(transport.build(
                        self.resource.as_ref().map(resource_proto),
                        self.scope.as_ref().map(scope_proto),
                    )?))
                }
                None => None,
            },
        };

        let metrics = Arc::new(InternalMetrics::default());

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
                    if let Some(client) = client.otlp_logs {
                        if let Err(e) = client
                            .send(otlp_logs, logs::encode_request, {
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
                                otlp_logs: logs,
                                otlp_traces: Vec::new(),
                                otlp_metrics: Vec::new(),
                            }));
                        }
                    }
                }

                if otlp_traces.len() > 0 {
                    if let Some(client) = client.otlp_traces {
                        if let Err(e) = client
                            .send(otlp_traces, traces::encode_request, {
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
                    if let Some(client) = client.otlp_metrics {
                        if let Err(e) = client
                            .send(otlp_metrics, metrics::encode_request, {
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
            otlp_logs,
            otlp_traces,
            otlp_metrics,
            metrics,
            sender,
        })
    }
}

fn resource_proto(resource: &Resource) -> PreEncoded {
    let protobuf = sval_protobuf::stream_to_protobuf(data::Resource {
        attributes: &data::PropsResourceAttributes(&resource.attributes),
        dropped_attribute_count: 0,
    });

    PreEncoded::Proto(protobuf)
}

fn scope_proto(scope: &Scope) -> PreEncoded {
    let protobuf = sval_protobuf::stream_to_protobuf(data::InstrumentationScope {
        name: &scope.name,
        version: &scope.version,
        attributes: &data::PropsInstrumentationScopeAttributes(&scope.attributes),
        dropped_attribute_count: 0,
    });

    PreEncoded::Proto(protobuf)
}

#[derive(Clone)]
pub struct OtlpClient {
    // TODO: Share the client when possible
    otlp_logs: Option<Arc<OtlpTransport>>,
    otlp_traces: Option<Arc<OtlpTransport>>,
    otlp_metrics: Option<Arc<OtlpTransport>>,
}

enum OtlpTransport {
    Http {
        http: HttpConnection,
        resource: Option<PreEncoded>,
        scope: Option<PreEncoded>,
    },
}

impl OtlpTransport {
    #[emit::span(rt: emit::runtime::internal(), arg: span, "send")]
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
        match self {
            OtlpTransport::Http {
                ref http,
                ref resource,
                ref scope,
            } => {
                let batch_size = batch.len();

                let res = match http
                    .send(encode(resource.as_ref(), scope.as_ref(), &batch)?)
                    .await
                {
                    Ok(res) => {
                        span.complete(|extent| {
                            emit::debug!(
                                rt: emit::runtime::internal(),
                                when: emit::filter::always(),
                                extent,
                                "OTLP batch of {batch_size} events responded {status_code}",
                                batch_size,
                                status_code: res.status(),
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
                                "OTLP batch of {batch_size} events failed to send: {err}",
                                batch_size,
                                err,
                            )
                        });

                        return Err(BatchError::retry(err, batch));
                    }
                };

                if let Some(decode) = decode {
                    let status = res.status();
                    let body = res.read_to_vec().await.map_err(|err| {
                        emit::warn!(
                            rt: emit::runtime::internal(),
                            "failed to read OTLP response: {err}",
                            err,
                        );

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
    }
}
