mod client;
pub mod data;
mod error;

pub use self::{client::*, error::*};

pub fn proto() -> OtlpClientBuilder {
    OtlpClientBuilder::proto()
}
