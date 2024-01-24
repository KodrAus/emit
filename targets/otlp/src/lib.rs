#![feature(stmt_expr_attributes, proc_macro_hygiene)]

mod client;
pub mod data;
mod error;

pub use self::{client::*, error::*};

pub fn new() -> OtlpBuilder {
    OtlpBuilder::new()
}

pub fn http(dst: impl Into<String>) -> OtlpTransportBuilder {
    OtlpTransportBuilder::http(dst)
}

pub fn logs_http_proto(dst: impl Into<String>) -> OtlpLogsBuilder {
    OtlpLogsBuilder::http_proto(dst)
}

pub fn logs_proto(transport: OtlpTransportBuilder) -> OtlpLogsBuilder {
    OtlpLogsBuilder::proto(transport)
}

pub fn traces_http_proto(dst: impl Into<String>) -> OtlpTracesBuilder {
    OtlpTracesBuilder::http_proto(dst)
}

pub fn traces_proto(transport: OtlpTransportBuilder) -> OtlpTracesBuilder {
    OtlpTracesBuilder::proto(transport)
}

pub fn metrics_http_proto(dst: impl Into<String>) -> OtlpMetricsBuilder {
    OtlpMetricsBuilder::http_proto(dst)
}

pub fn metrics_proto(transport: OtlpTransportBuilder) -> OtlpMetricsBuilder {
    OtlpMetricsBuilder::proto(transport)
}
