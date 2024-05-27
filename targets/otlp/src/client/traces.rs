use std::{fmt, sync::Arc};

use crate::{
    data::traces::{self, TracesEventEncoder, TracesRequestEncoder},
    internal_metrics::InternalMetrics,
    Encoding, Error, OtlpTransportBuilder,
};

use super::{
    encode_resource, ClientEventEncoder, ClientRequestEncoder, OtlpTransport, Protocol, Resource,
};

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

    pub(in crate::client) fn build(
        self,
        metrics: Arc<InternalMetrics>,
        resource: Option<&Resource>,
    ) -> Result<
        (
            ClientEventEncoder<TracesEventEncoder>,
            OtlpTransport<TracesRequestEncoder>,
        ),
        Error,
    > {
        Ok((
            ClientEventEncoder::new(self.encoding, self.event_encoder),
            self.transport.build(
                metrics.clone(),
                resource
                    .as_ref()
                    .map(|resource| encode_resource(self.encoding, resource)),
                ClientRequestEncoder::new(self.encoding, self.request_encoder),
            )?,
        ))
    }
}
