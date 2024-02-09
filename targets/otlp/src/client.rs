use emit_batcher::BatchError;
use std::{collections::HashMap, fmt, sync::Arc, time::Duration};

use crate::{
    data::{self, default_message_formatter, logs, metrics, traces, PreEncoded},
    Error,
};

use self::http::HttpConnection;

mod http;

pub struct Otlp {
    logs: Option<logs::EventEncoder>,
    traces: Option<traces::EventEncoder>,
    metrics: Option<metrics::EventEncoder>,
    sender: emit_batcher::Sender<Channel<PreEncoded>>,
}

impl emit::emitter::Emitter for Otlp {
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

pub struct OtlpBuilder {
    resource: Option<Resource>,
    scope: Option<Scope>,
    logs: Option<OtlpLogsBuilder>,
    traces: Option<OtlpTracesBuilder>,
    metrics: Option<OtlpMetricsBuilder>,
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
            encoder: logs::EventEncoder {
                body: default_message_formatter(),
            },
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
            encoder: traces::EventEncoder {
                name: default_message_formatter(),
            },
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
            encoder: metrics::EventEncoder {
                name: default_message_formatter(),
            },
            encoding: Encoding::Proto,
            transport,
        }
    }

    pub fn http_proto(dst: impl Into<String>) -> Self {
        Self::proto(OtlpTransportBuilder::http(dst))
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
            logs: None,
            traces: None,
            metrics: None,
        }
    }

    pub fn logs_http_proto(self, dst: impl Into<String>) -> Self {
        self.logs(OtlpLogsBuilder::http_proto(dst))
    }

    pub fn logs(mut self, builder: OtlpLogsBuilder) -> Self {
        self.logs = Some(builder);
        self
    }

    pub fn traces_http_proto(self, dst: impl Into<String>) -> Self {
        self.traces(OtlpTracesBuilder::http_proto(dst))
    }

    pub fn traces(mut self, builder: OtlpTracesBuilder) -> Self {
        self.traces = Some(builder);
        self
    }

    pub fn metrics_http_proto(self, dst: impl Into<String>) -> Self {
        self.metrics(OtlpMetricsBuilder::http_proto(dst))
    }

    pub fn metrics(mut self, builder: OtlpMetricsBuilder) -> Self {
        self.metrics = Some(builder);
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

        let mut logs = None;
        let mut traces = None;
        let mut metrics = None;

        let client = OtlpClient {
            logs: match self.logs {
                Some(OtlpLogsBuilder {
                    encoder,
                    encoding: Encoding::Proto,
                    transport,
                }) => {
                    logs = Some(encoder);
                    Some(Arc::new(transport.build(
                        self.resource.as_ref().map(resource_proto),
                        self.scope.as_ref().map(scope_proto),
                    )?))
                }
                None => None,
            },
            traces: match self.traces {
                Some(OtlpTracesBuilder {
                    encoder,
                    encoding: Encoding::Proto,
                    transport,
                }) => {
                    traces = Some(encoder);
                    Some(Arc::new(transport.build(
                        self.resource.as_ref().map(resource_proto),
                        self.scope.as_ref().map(scope_proto),
                    )?))
                }
                None => None,
            },
            metrics: match self.metrics {
                Some(OtlpMetricsBuilder {
                    encoder,
                    encoding: Encoding::Proto,
                    transport,
                }) => {
                    metrics = Some(encoder);
                    Some(Arc::new(transport.build(
                        self.resource.as_ref().map(resource_proto),
                        self.scope.as_ref().map(scope_proto),
                    )?))
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

        Ok(Otlp {
            logs,
            traces,
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
    logs: Option<Arc<OtlpTransport>>,
    traces: Option<Arc<OtlpTransport>>,
    metrics: Option<Arc<OtlpTransport>>,
}

enum OtlpTransport {
    Http {
        http: HttpConnection,
        resource: Option<PreEncoded>,
        scope: Option<PreEncoded>,
    },
}

impl OtlpTransport {
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

        // TODO: Function to start a span
        let ctxt = emit::frame::Frame::new_push(
            rt.ctxt(),
            emit::props! {
                trace_id: rt.gen_trace_id(),
                span_id: rt.gen_span_id(),
            },
        );

        ctxt.with_future(async move {
            match self {
                OtlpTransport::Http {
                    ref http,
                    ref resource,
                    ref scope,
                } => {
                    let batch_size = batch.len();

                    let timer = emit::timer::Timer::start(rt);

                    let res = http
                        .send(encode(resource.as_ref(), scope.as_ref(), &batch)?)
                        .await
                        .map_err(|err| {
                            rt.emit(&emit::warn_event!(
                                extent: timer,
                                "OTLP batch of {batch_size} events failed to send: {err}",
                                batch_size,
                                err,
                            ));

                            BatchError::retry(err, batch)
                        })?;

                    rt.emit(&emit::debug_event!(
                        extent: timer,
                        "OTLP batch of {batch_size} events responded {status_code}",
                        batch_size,
                        status_code: res.status(),
                    ));

                    if let Some(decode) = decode {
                        let status = res.status();
                        let body = res.read_to_vec().await.map_err(|err| {
                            rt.emit(&emit::warn_event!(
                                "failed to read OTLP response: {err}",
                                err,
                            ));

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
        })
        .await
    }
}
