use sval_derive::Value;

use crate::data::{stream_attributes, stream_field, AnyValue, KeyValue};

#[derive(Value)]
#[repr(i32)]
pub enum StatusCode {
    Unset = 0,
    Ok = 1,
    Error = 2,
}

#[derive(Value)]
#[repr(i32)]
pub enum SpanKind {
    Unspecified = 0,
    Internal = 1,
    Server = 2,
    Client = 3,
    Producer = 4,
    Consumer = 5,
}

#[derive(Value)]
pub struct Span<'a, N: ?Sized = str, A: ?Sized = InlineSpanAttributes<'a>> {
    #[sval(label = "name", index = 5)]
    pub name: &'a N,
    #[sval(label = "kind", index = 6)]
    pub kind: SpanKind,
    #[sval(
        label = "startTimeUnixNano",
        index = 7,
        data_tag = "sval_protobuf::tags::PROTOBUF_I64"
    )]
    pub start_time_unix_nano: u64,
    #[sval(
        label = "endTimeUnixNano",
        index = 8,
        data_tag = "sval_protobuf::tags::PROTOBUF_I64"
    )]
    pub end_time_unix_nano: u64,
    #[sval(label = "droppedAttributesCount", index = 10)]
    pub dropped_attributes_count: u32,
    #[sval(flatten)]
    pub attributes: &'a A,
}

const SPAN_ATTRIBUTES_LABEL: sval::Label =
    sval::Label::new("attributes").with_tag(&sval::tags::VALUE_IDENT);
const SPAN_TRACE_ID_LABEL: sval::Label =
    sval::Label::new("traceId").with_tag(&sval::tags::VALUE_IDENT);
const SPAN_SPAN_ID_LABEL: sval::Label =
    sval::Label::new("spanId").with_tag(&sval::tags::VALUE_IDENT);
const SPAN_PARENT_SPAN_ID_LABEL: sval::Label =
    sval::Label::new("parentSpanId").with_tag(&sval::tags::VALUE_IDENT);
const SPAN_STATUS_LABEL: sval::Label =
    sval::Label::new("status").with_tag(&sval::tags::VALUE_IDENT);

const SPAN_EVENTS_LABEL: sval::Label =
    sval::Label::new("events").with_tag(&sval::tags::VALUE_IDENT);

const SPAN_ATTRIBUTES_INDEX: sval::Index = sval::Index::new(9);
const SPAN_TRACE_ID_INDEX: sval::Index = sval::Index::new(1);
const SPAN_SPAN_ID_INDEX: sval::Index = sval::Index::new(2);
const SPAN_PARENT_SPAN_ID_INDEX: sval::Index = sval::Index::new(4);
const SPAN_STATUS_INDEX: sval::Index = sval::Index::new(15);
const SPAN_EVENTS_INDEX: sval::Index = sval::Index::new(11);

#[derive(Value)]
pub struct InlineSpanAttributes<'a, E: ?Sized = [Event<'a>]> {
    #[sval(label = SPAN_ATTRIBUTES_LABEL, index = SPAN_ATTRIBUTES_INDEX)]
    pub attributes: &'a [KeyValue<&'a str, &'a AnyValue<'a>>],
    #[sval(label = SPAN_TRACE_ID_LABEL, index = SPAN_TRACE_ID_INDEX)]
    pub trace_id: &'a sval::BinaryArray<16>,
    #[sval(label = SPAN_SPAN_ID_LABEL, index = SPAN_SPAN_ID_INDEX)]
    pub span_id: &'a sval::BinaryArray<8>,
    #[sval(label = SPAN_PARENT_SPAN_ID_LABEL, index = SPAN_PARENT_SPAN_ID_INDEX)]
    pub parent_span_id: &'a sval::BinaryArray<8>,
    #[sval(label = SPAN_STATUS_LABEL, index = SPAN_STATUS_INDEX)]
    pub status: Status<'a>,
    #[sval(label = SPAN_EVENTS_LABEL, index = SPAN_EVENTS_INDEX)]
    pub events: &'a E,
}

pub struct PropsSpanAttributes<P> {
    pub time_unix_nano: u64,
    pub props: P,
}

impl<P: emit_core::props::Props> sval::Value for PropsSpanAttributes<P> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        let mut trace_id = [0; 32];
        let mut span_id = [0; 16];
        let mut parent_span_id = [0; 16];
        let mut level = emit_core::level::Level::default();
        let mut has_err = false;

        stream.record_tuple_begin(None, None, None, None)?;

        stream_field(
            &mut *stream,
            &SPAN_ATTRIBUTES_LABEL,
            &SPAN_ATTRIBUTES_INDEX,
            |stream| {
                stream_attributes(stream, &self.props, |k, v| match k.as_str() {
                    emit_core::well_known::LVL_KEY => {
                        level = v.to_level().unwrap_or_default();
                        true
                    }
                    emit_core::well_known::SPAN_ID_KEY => {
                        span_id = v
                            .to_span_id()
                            .map(|span_id| span_id.to_hex())
                            .unwrap_or_default();
                        true
                    }
                    emit_core::well_known::SPAN_PARENT_KEY => {
                        parent_span_id = v
                            .to_span_id()
                            .map(|parent_span_id| parent_span_id.to_hex())
                            .unwrap_or_default();
                        true
                    }
                    emit_core::well_known::TRACE_ID_KEY => {
                        trace_id = v
                            .to_trace_id()
                            .map(|trace_id| trace_id.to_hex())
                            .unwrap_or_default();
                        true
                    }
                    emit_core::well_known::ERR_KEY => {
                        has_err = true;
                        false
                    }
                    _ => false,
                })
            },
        )?;

        if trace_id != [0; 32] {
            stream_field(
                &mut *stream,
                &SPAN_TRACE_ID_LABEL,
                &SPAN_TRACE_ID_INDEX,
                |stream| {
                    stream.binary_begin(Some(32))?;
                    stream.binary_fragment_computed(&trace_id)?;
                    stream.binary_end()
                },
            )?;
        }

        if span_id != [0; 16] {
            stream_field(
                &mut *stream,
                &SPAN_SPAN_ID_LABEL,
                &SPAN_SPAN_ID_INDEX,
                |stream| {
                    stream.binary_begin(Some(16))?;
                    stream.binary_fragment_computed(&span_id)?;
                    stream.binary_end()
                },
            )?;
        }

        if parent_span_id != [0; 16] {
            stream_field(
                &mut *stream,
                &SPAN_PARENT_SPAN_ID_LABEL,
                &SPAN_PARENT_SPAN_ID_INDEX,
                |stream| {
                    stream.binary_begin(Some(16))?;
                    stream.binary_fragment_computed(&parent_span_id)?;
                    stream.binary_end()
                },
            )?;
        }

        if has_err {
            let err = self.props.get(emit_core::well_known::ERR_KEY).unwrap();

            stream_field(
                &mut *stream,
                &SPAN_EVENTS_LABEL,
                &SPAN_EVENTS_INDEX,
                |stream| {
                    stream.value_computed(&[Event {
                        name: "exception",
                        time_unix_nano: self.time_unix_nano,
                        dropped_attributes_count: 0,
                        attributes: &[KeyValue {
                            key: "exception.message",
                            value: AnyValue::<_, (), (), ()>::String(sval::Display::new_borrowed(
                                &err,
                            )),
                        }],
                    }])
                },
            )?;

            let status = Status {
                code: StatusCode::Error,
                message: sval::Display::new_borrowed(&err),
            };

            stream_field(
                &mut *stream,
                &SPAN_STATUS_LABEL,
                &SPAN_STATUS_INDEX,
                |stream| stream.value_computed(&status),
            )?;
        }

        stream.record_tuple_end(None, None, None)
    }
}

#[derive(Value)]
pub struct Status<'a, M: ?Sized = str> {
    #[sval(label = "message", index = 2)]
    pub message: &'a M,
    #[sval(label = "code", index = 3)]
    pub code: StatusCode,
}

#[derive(Value)]
pub struct Event<'a, N: ?Sized = str, A: ?Sized = InlineEventAttributes<'a>> {
    #[sval(label = "name", index = 2)]
    pub name: &'a N,
    #[sval(
        label = "timeUnixNano",
        index = 1,
        data_tag = "sval_protobuf::tags::PROTOBUF_I64"
    )]
    pub time_unix_nano: u64,
    #[sval(label = "droppedAttributesCount", index = 4)]
    pub dropped_attributes_count: u32,
    #[sval(flatten)]
    pub attributes: &'a A,
}

const EVENT_ATTRIBUTES_LABEL: sval::Label =
    sval::Label::new("attributes").with_tag(&sval::tags::VALUE_IDENT);

const EVENT_ATTRIBUTES_INDEX: sval::Index = sval::Index::new(3);

#[derive(Value)]
pub struct InlineEventAttributes<'a> {
    #[sval(label = EVENT_ATTRIBUTES_LABEL, index = EVENT_ATTRIBUTES_INDEX)]
    pub attributes: &'a [KeyValue<&'a str, &'a AnyValue<'a>>],
}

pub struct PropsEventAttributes<P>(pub P);

impl<P: emit_core::props::Props> sval::Value for PropsEventAttributes<P> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.record_tuple_begin(None, None, None, None)?;

        stream_field(
            &mut *stream,
            &EVENT_ATTRIBUTES_LABEL,
            &EVENT_ATTRIBUTES_INDEX,
            |stream| stream_attributes(stream, &self.0, |_, _| false),
        )?;

        stream.record_tuple_end(None, None, None)
    }
}