use std::{fmt, sync::Arc};

use crate::{
    data::logs::{self, LogsEventEncoder, LogsRequestEncoder},
    internal_metrics::InternalMetrics,
    Encoding, Error, OtlpTransportBuilder,
};

use super::{
    encode_resource, ClientEventEncoder, ClientRequestEncoder, OtlpTransport, Protocol, Resource,
};

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

    pub(in crate::client) fn build(
        self,
        metrics: Arc<InternalMetrics>,
        resource: Option<&Resource>,
    ) -> Result<
        (
            ClientEventEncoder<LogsEventEncoder>,
            OtlpTransport<LogsRequestEncoder>,
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
