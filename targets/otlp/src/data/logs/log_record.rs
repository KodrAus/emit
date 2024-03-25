use sval_derive::Value;

use crate::data::{stream_attributes, stream_field, AnyValue, KeyValue};

#[derive(Value)]
#[repr(i32)]
pub enum SeverityNumber {
    Debug = 5,
    Info = 9,
    Warn = 13,
    Error = 17,
}

#[derive(Value)]
pub struct LogRecord<'a, B: ?Sized = AnyValue<'a>, A: ?Sized = InlineLogRecordAttributes<'a>> {
    #[sval(
        label = "timeUnixNano",
        index = 1,
        data_tag = "sval_protobuf::tags::PROTOBUF_I64"
    )]
    pub time_unix_nano: u64,
    #[sval(
        label = "observedTimeUnixNano",
        index = 11,
        data_tag = "sval_protobuf::tags::PROTOBUF_I64"
    )]
    pub observed_time_unix_nano: u64,
    #[sval(label = "droppedAttributesCount", index = 7)]
    pub dropped_attributes_count: u32,
    #[sval(
        label = "flags",
        index = 8,
        data_tag = "sval_protobuf::tags::PROTOBUF_I32"
    )]
    pub flags: u32,
    #[sval(label = "body", index = 5)]
    pub body: &'a B,
    #[sval(flatten)]
    pub attributes: &'a A,
}

const LOG_RECORD_SEVERITY_NUMBER_LABEL: sval::Label =
    sval::Label::new("severityNumber").with_tag(&sval::tags::VALUE_IDENT);
const LOG_RECORD_SEVERITY_TEXT_LABEL: sval::Label =
    sval::Label::new("severityText").with_tag(&sval::tags::VALUE_IDENT);
const LOG_RECORD_ATTRIBUTES_LABEL: sval::Label =
    sval::Label::new("attributes").with_tag(&sval::tags::VALUE_IDENT);
const LOG_RECORD_TRACE_ID_LABEL: sval::Label =
    sval::Label::new("traceId").with_tag(&sval::tags::VALUE_IDENT);
const LOG_RECORD_SPAN_ID_LABEL: sval::Label =
    sval::Label::new("spanId").with_tag(&sval::tags::VALUE_IDENT);

const LOG_RECORD_SEVERITY_NUMBER_INDEX: sval::Index = sval::Index::new(2);
const LOG_RECORD_SEVERITY_TEXT_INDEX: sval::Index = sval::Index::new(3);
const LOG_RECORD_ATTRIBUTES_INDEX: sval::Index = sval::Index::new(6);
const LOG_RECORD_TRACE_ID_INDEX: sval::Index = sval::Index::new(9);
const LOG_RECORD_SPAN_ID_INDEX: sval::Index = sval::Index::new(10);

#[derive(Value)]
pub struct InlineLogRecordAttributes<'a> {
    #[sval(label = LOG_RECORD_SEVERITY_NUMBER_LABEL, index = LOG_RECORD_SEVERITY_NUMBER_INDEX)]
    pub severity_number: SeverityNumber,
    #[sval(label = LOG_RECORD_SEVERITY_TEXT_LABEL, index = LOG_RECORD_SEVERITY_TEXT_INDEX)]
    pub severity_text: &'a str,
    #[sval(label = LOG_RECORD_ATTRIBUTES_LABEL, index = LOG_RECORD_ATTRIBUTES_INDEX)]
    pub attributes: &'a [KeyValue<&'a str, &'a AnyValue<'a>>],
    #[sval(label = LOG_RECORD_TRACE_ID_LABEL, index = LOG_RECORD_TRACE_ID_INDEX)]
    pub trace_id: &'a sval::BinaryArray<16>,
    #[sval(label = LOG_RECORD_SPAN_ID_LABEL, index = LOG_RECORD_SPAN_ID_INDEX)]
    pub span_id: &'a sval::BinaryArray<8>,
}

pub struct PropsLogRecordAttributes<P>(pub P);

impl<P: emit::props::Props> sval::Value for PropsLogRecordAttributes<P> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        let mut trace_id = [0; 16];
        let mut span_id = [0; 8];
        let mut level = emit::level::Level::default();

        stream.record_tuple_begin(None, None, None, None)?;

        stream_field(
            &mut *stream,
            &LOG_RECORD_ATTRIBUTES_LABEL,
            &LOG_RECORD_ATTRIBUTES_INDEX,
            |stream| {
                stream_attributes(stream, &self.0, |k, v| match k.get() {
                    emit::well_known::KEY_LVL => {
                        level = v.by_ref().cast::<emit::Level>().unwrap_or_default();
                        true
                    }
                    emit::well_known::KEY_SPAN_ID => {
                        span_id = v
                            .by_ref()
                            .cast::<emit::trace::SpanId>()
                            .map(|span_id| span_id.to_u64().to_be_bytes())
                            .unwrap_or_default();
                        true
                    }
                    emit::well_known::KEY_TRACE_ID => {
                        trace_id = v
                            .by_ref()
                            .cast::<emit::trace::TraceId>()
                            .map(|trace_id| trace_id.to_u128().to_be_bytes())
                            .unwrap_or_default();
                        true
                    }
                    _ => false,
                })
            },
        )?;

        let severity_number = match level {
            emit::level::Level::Debug => SeverityNumber::Debug as i32,
            emit::level::Level::Info => SeverityNumber::Info as i32,
            emit::level::Level::Warn => SeverityNumber::Warn as i32,
            emit::level::Level::Error => SeverityNumber::Error as i32,
        };

        stream_field(
            &mut *stream,
            &LOG_RECORD_SEVERITY_NUMBER_LABEL,
            &LOG_RECORD_SEVERITY_NUMBER_INDEX,
            |stream| stream.i32(severity_number),
        )?;

        stream_field(
            &mut *stream,
            &LOG_RECORD_SEVERITY_TEXT_LABEL,
            &LOG_RECORD_SEVERITY_TEXT_INDEX,
            |stream| sval::stream_display(stream, level),
        )?;

        if trace_id != [0; 16] {
            stream_field(
                &mut *stream,
                &LOG_RECORD_TRACE_ID_LABEL,
                &LOG_RECORD_TRACE_ID_INDEX,
                |stream| stream.value_computed(&sval::BinaryArray::new(&trace_id)),
            )?;
        }

        if span_id != [0; 8] {
            stream_field(
                &mut *stream,
                &LOG_RECORD_SPAN_ID_LABEL,
                &LOG_RECORD_SPAN_ID_INDEX,
                |stream| stream.value_computed(&sval::BinaryArray::new(&span_id)),
            )?;
        }

        stream.record_tuple_end(None, None, None)
    }
}
