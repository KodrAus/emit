mod client;
pub mod data;
mod error;

pub use self::{client::*, error::*};
mod logs;
mod traces;

pub fn proto() -> OtlpClientBuilder {
    OtlpClientBuilder::proto()
}
