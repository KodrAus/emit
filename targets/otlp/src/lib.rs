/*!
Emit diagnostic events via the OpenTelemetry Protocol (OTLP).

This library provides an [`emit::Emitter`] that writes OTLP directly. If you need to integrate [`emit`] with the OpenTelemetry SDK, see `emit-opentelemetry`.

It supports the following transports:

- HTTP/HTTPS + JSON.
- HTTP/HTTPS + Protocol Buffer.
- gRPC + Protocol Buffers.

It supports the following compression:

- None.
- gzip.

It supports the following signals:

- Logs.
- Traces.
- Metrics.

The emitter is asynchronous. Diagnostic events are serialized on-thread and then put into a queue to be flushed in the background. Connections are made using [`hyper`] over [`tokio`].

# Getting started

Add `emit` and `emit_otlp` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "0.0.0"

[dependencies.emit_otlp]
version = "0.0.0"
```

## Configuring for gRPC + Protocol Buffers

Initialize `emit` at the start of your `main.rs` using an OTLP emitter:

```
fn main() {
    let rt = emit::setup()
        .emit_to({
            let otlp = emit_otlp::new()
                // Add required resource properties for OTLP
                .resource(emit::props! {
                    #[emit::key("service.name")]
                    service_name: env!("CARGO_PKG_NAME"),
                    #[emit::key("telemetry.sdk.language")]
                    language: "rust",
                    #[emit::key("telemetry.sdk.name")]
                    sdk: emit_otlp::telemetry_sdk_name(),
                    #[emit::key("telemetry.sdk.version")]
                    version: emit_otlp::telemetry_sdk_version(),
                })
                // Configure endpoints for logs/traces/metrics using gRPC + protobuf
                .logs(emit_otlp::logs_grpc_proto("http://localhost:4319"))
                .traces(emit_otlp::traces_grpc_proto("http://localhost:4319"))
                .metrics(emit_otlp::metrics_grpc_proto("http://localhost:4319"))
                .spawn()
                .unwrap();

            otlp
        })
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));
}
```

# How events map to OTLP

Events that are [`mod@emit::span`]s will be emitted as OTLP traces. Events that are [`mod@emit::metric`]s will be emitted as OTLP metrics. All other events will be emitted as OTLP logs.

You don't always need to configure endpoints for logs, traces, and metrics. The only necessary one is logs. If traces or metrics aren't configured then those events will fall back to being treated as logs. If logs aren't configured then events that can't be treated as traces or metrics will be discarded.

The [`emit::Event::module`] is used as the name of the instrumentation scope.

# Limitations

This library is not an alternative to the OpenTelemetry SDK. It's specifically targeted at emitting diagnostic events to OTLP-compatible services. It has some intentional limitations:

- **No propagation.** This is the responsibility of the application to manage.
- **No histogram metrics.** `emit`'s data model for metrics is simplistic compared to OpenTelemetry's, so it doesn't support histograms or exponential histograms.
- **No span events.** Only the conventional exception event is supported. Standalone log events are not converted into span events. They're sent via the logs endpoint instead.

# Troubleshooting


*/

#[macro_use]
mod internal_metrics;
mod client;
pub mod data;
mod error;

pub use self::{client::*, error::*};

pub const fn telemetry_sdk_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

pub const fn telemetry_sdk_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn new() -> OtlpBuilder {
    OtlpBuilder::new()
}

pub fn grpc(dst: impl Into<String>) -> OtlpTransportBuilder {
    OtlpTransportBuilder::grpc(dst)
}

pub fn http(dst: impl Into<String>) -> OtlpTransportBuilder {
    OtlpTransportBuilder::http(dst)
}

pub fn logs_grpc_proto(dst: impl Into<String>) -> OtlpLogsBuilder {
    OtlpLogsBuilder::grpc_proto(dst)
}

pub fn logs_http_proto(dst: impl Into<String>) -> OtlpLogsBuilder {
    OtlpLogsBuilder::http_proto(dst)
}

pub fn logs_http_json(dst: impl Into<String>) -> OtlpLogsBuilder {
    OtlpLogsBuilder::http_json(dst)
}

pub fn logs_proto(transport: OtlpTransportBuilder) -> OtlpLogsBuilder {
    OtlpLogsBuilder::proto(transport)
}

pub fn logs_json(transport: OtlpTransportBuilder) -> OtlpLogsBuilder {
    OtlpLogsBuilder::json(transport)
}

pub fn traces_grpc_proto(dst: impl Into<String>) -> OtlpTracesBuilder {
    OtlpTracesBuilder::grpc_proto(dst)
}

pub fn traces_http_proto(dst: impl Into<String>) -> OtlpTracesBuilder {
    OtlpTracesBuilder::http_proto(dst)
}

pub fn traces_http_json(dst: impl Into<String>) -> OtlpTracesBuilder {
    OtlpTracesBuilder::http_json(dst)
}

pub fn traces_proto(transport: OtlpTransportBuilder) -> OtlpTracesBuilder {
    OtlpTracesBuilder::proto(transport)
}

pub fn traces_json(transport: OtlpTransportBuilder) -> OtlpTracesBuilder {
    OtlpTracesBuilder::json(transport)
}

pub fn metrics_grpc_proto(dst: impl Into<String>) -> OtlpMetricsBuilder {
    OtlpMetricsBuilder::grpc_proto(dst)
}

pub fn metrics_http_proto(dst: impl Into<String>) -> OtlpMetricsBuilder {
    OtlpMetricsBuilder::http_proto(dst)
}

pub fn metrics_http_json(dst: impl Into<String>) -> OtlpMetricsBuilder {
    OtlpMetricsBuilder::http_json(dst)
}

pub fn metrics_proto(transport: OtlpTransportBuilder) -> OtlpMetricsBuilder {
    OtlpMetricsBuilder::proto(transport)
}

pub fn metrics_json(transport: OtlpTransportBuilder) -> OtlpMetricsBuilder {
    OtlpMetricsBuilder::json(transport)
}
