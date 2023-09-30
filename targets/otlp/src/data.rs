use sval_derive::Value;
use sval_protobuf::buf::ProtoBuf;

mod any_value;
mod export_logs_service;
mod instrumentation_scope;
mod log_record;
mod resource;

pub use self::{
    any_value::*, export_logs_service::*, instrumentation_scope::*, log_record::*, resource::*,
};

#[derive(Value)]
#[sval(dynamic)]
pub(crate) enum PreEncoded {
    Proto(ProtoBuf),
}
