use std::borrow::Cow;

use bytes::Buf;
use sval_derive::Value;
use sval_protobuf::buf::{ProtoBuf, ProtoBufCursor};

mod any_value;
mod export_logs_service;
mod instrumentation_scope;
mod log_record;
mod resource;

#[cfg(feature = "grpc")]
pub(crate) mod generated;

pub(crate) use self::{
    any_value::*, export_logs_service::*, instrumentation_scope::*, log_record::*, resource::*,
};

#[derive(Value)]
#[sval(dynamic)]
pub(crate) enum PreEncoded {
    Proto(ProtoBuf),
}

impl PreEncoded {
    pub fn into_cursor(self) -> PreEncodedCursor {
        match self {
            PreEncoded::Proto(buf) => PreEncodedCursor::Proto(buf.into_cursor()),
        }
    }

    pub fn to_vec(&self) -> Cow<[u8]> {
        match self {
            PreEncoded::Proto(buf) => buf.to_vec(),
        }
    }
}

pub(crate) enum PreEncodedCursor {
    Proto(ProtoBufCursor),
}

impl Buf for PreEncodedCursor {
    fn remaining(&self) -> usize {
        match self {
            PreEncodedCursor::Proto(cursor) => cursor.remaining(),
        }
    }

    fn chunk(&self) -> &[u8] {
        match self {
            PreEncodedCursor::Proto(cursor) => cursor.chunk(),
        }
    }

    fn advance(&mut self, cnt: usize) {
        match self {
            PreEncodedCursor::Proto(cursor) => cursor.advance(cnt),
        }
    }
}
