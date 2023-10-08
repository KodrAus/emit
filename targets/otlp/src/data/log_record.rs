use sval_derive::Value;

use super::{attributes::stream_attributes, AnyValue, KeyValue};

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

pub(crate) struct EmitLogRecordAttributes<P>(pub P);

impl<P: emit_core::props::Props> sval::Value for EmitLogRecordAttributes<P> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        let mut trace_id = [0; 32];
        let mut span_id = [0; 16];
        let mut level = emit_core::level::Level::default();

        stream.record_tuple_begin(None, None, None, None)?;

        stream.record_tuple_value_begin(
            None,
            &LOG_RECORD_ATTRIBUTES_LABEL,
            &LOG_RECORD_ATTRIBUTES_INDEX,
        )?;
        stream_attributes(&mut *stream, &self.0, |k, v| match k.as_str() {
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
            emit_core::well_known::TRACE_ID_KEY => {
                trace_id = v
                    .to_trace_id()
                    .map(|trace_id| trace_id.to_hex())
                    .unwrap_or_default();
                true
            }
            _ => false,
        })?;
        stream.record_tuple_value_end(
            None,
            &LOG_RECORD_ATTRIBUTES_LABEL,
            &LOG_RECORD_ATTRIBUTES_INDEX,
        )?;

        let severity_number = match level {
            emit_core::level::Level::Debug => SeverityNumber::Debug as i32,
            emit_core::level::Level::Info => SeverityNumber::Info as i32,
            emit_core::level::Level::Warn => SeverityNumber::Warn as i32,
            emit_core::level::Level::Error => SeverityNumber::Error as i32,
        };

        stream.record_tuple_value_begin(
            None,
            &LOG_RECORD_SEVERITY_NUMBER_LABEL,
            &LOG_RECORD_SEVERITY_NUMBER_INDEX,
        )?;
        stream.i32(severity_number)?;
        stream.record_tuple_value_end(
            None,
            &LOG_RECORD_SEVERITY_NUMBER_LABEL,
            &LOG_RECORD_SEVERITY_NUMBER_INDEX,
        )?;

        stream.record_tuple_value_begin(
            None,
            &LOG_RECORD_SEVERITY_TEXT_LABEL,
            &LOG_RECORD_SEVERITY_TEXT_INDEX,
        )?;
        sval::stream_display(&mut *stream, level)?;
        stream.record_tuple_value_end(
            None,
            &LOG_RECORD_SEVERITY_TEXT_LABEL,
            &LOG_RECORD_SEVERITY_TEXT_INDEX,
        )?;

        if trace_id != [0; 32] {
            stream.record_tuple_value_begin(
                None,
                &LOG_RECORD_TRACE_ID_LABEL,
                &LOG_RECORD_TRACE_ID_INDEX,
            )?;
            stream.binary_begin(Some(32))?;
            stream.binary_fragment_computed(&trace_id)?;
            stream.binary_end()?;
            stream.record_tuple_value_end(
                None,
                &LOG_RECORD_TRACE_ID_LABEL,
                &LOG_RECORD_TRACE_ID_INDEX,
            )?;
        }

        if span_id != [0; 16] {
            stream.record_tuple_value_begin(
                None,
                &LOG_RECORD_SPAN_ID_LABEL,
                &LOG_RECORD_SPAN_ID_INDEX,
            )?;
            stream.binary_begin(Some(16))?;
            stream.binary_fragment_computed(&span_id)?;
            stream.binary_end()?;
            stream.record_tuple_value_end(
                None,
                &LOG_RECORD_SPAN_ID_LABEL,
                &LOG_RECORD_SPAN_ID_INDEX,
            )?;
        }

        stream.record_tuple_end(None, None, None)
    }
}
