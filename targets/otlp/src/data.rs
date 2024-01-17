use std::{borrow::Cow, collections::HashSet, fmt, ops::ControlFlow};

use bytes::Buf;
use sval_derive::Value;
use sval_protobuf::buf::{ProtoBuf, ProtoBufCursor};

pub mod logs;
pub mod traces;

mod any_value;
mod instrumentation_scope;
mod resource;

#[cfg(feature = "grpc")]
pub(crate) mod generated;

pub use self::{any_value::*, instrumentation_scope::*, resource::*};

#[derive(Value, Clone)]
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

pub(crate) fn stream_field<'sval, S: sval::Stream<'sval> + ?Sized>(
    stream: &mut S,
    label: &sval::Label,
    index: &sval::Index,
    field: impl FnOnce(&mut S) -> sval::Result,
) -> sval::Result {
    stream.record_tuple_value_begin(None, label, index)?;
    field(&mut *stream)?;
    stream.record_tuple_value_end(None, label, index)
}

pub(crate) fn stream_attributes<'sval>(
    stream: &mut (impl sval::Stream<'sval> + ?Sized),
    props: &'sval impl emit_core::props::Props,
    mut for_each: impl FnMut(&emit_core::str::Str, &emit_core::value::Value) -> bool,
) -> sval::Result {
    stream.seq_begin(None)?;

    let mut seen = HashSet::new();
    props.for_each(|k, v| {
        if !for_each(&k, &v) && seen.insert(k.to_cow()) {
            stream
                .seq_value_begin()
                .map(|_| ControlFlow::Continue(()))
                .unwrap_or(ControlFlow::Break(()))?;

            sval_ref::stream_ref(
                &mut *stream,
                KeyValue {
                    key: k,
                    value: EmitValue(v),
                },
            )
            .map(|_| ControlFlow::Continue(()))
            .unwrap_or(ControlFlow::Break(()))?;

            stream
                .seq_value_end()
                .map(|_| ControlFlow::Continue(()))
                .unwrap_or(ControlFlow::Break(()))?;
        }

        ControlFlow::Continue(())
    });

    stream.seq_end()
}

pub(crate) type MessageFormatter = dyn Fn(
        &emit_core::event::Event<&dyn emit_core::props::ErasedProps>,
        &mut fmt::Formatter,
    ) -> fmt::Result
    + Send
    + Sync;

pub(crate) fn default_message_formatter() -> Box<MessageFormatter> {
    Box::new(|evt, f| write!(f, "{}", evt.msg()))
}

pub(crate) struct MessageRenderer<'a, P> {
    pub fmt: &'a MessageFormatter,
    pub evt: &'a emit_core::event::Event<'a, P>,
}

impl<'a, P: emit_core::props::Props> fmt::Display for MessageRenderer<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.fmt)(&self.evt.erase(), f)
    }
}
