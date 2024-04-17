#![feature(stmt_expr_attributes, proc_macro_hygiene)]

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
