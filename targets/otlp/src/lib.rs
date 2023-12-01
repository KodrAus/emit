mod client;
pub mod data;
mod error;

pub use self::{client::*, error::*};

pub fn proto() -> OtlpClientBuilder {
    OtlpClientBuilder::proto()
}

pub fn logs_http(dst: impl Into<String>) -> OtlpLogsBuilder {
    OtlpLogsBuilder::http(dst)
}

pub fn traces_http(dst: impl Into<String>) -> OtlpTracesBuilder {
    OtlpTracesBuilder::http(dst)
}
