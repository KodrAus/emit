use std::{collections::HashSet, ops::ControlFlow};

use sval_derive::Value;

use super::{AnyValue, EmitValue, KeyValue};

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
    #[sval(index = 1, data_tag = "sval_protobuf::tags::PROTOBUF_I64")]
    pub time_unix_nano: u64,
    #[sval(index = 11, data_tag = "sval_protobuf::tags::PROTOBUF_I64")]
    pub observed_time_unix_nano: u64,
    #[sval(index = 7)]
    pub dropped_attributes_count: u32,
    #[sval(index = 8, data_tag = "sval_protobuf::tags::PROTOBUF_I32")]
    pub flags: u32,
    #[sval(index = 5)]
    pub body: &'a B,
    #[sval(flatten)]
    pub attributes: &'a A,
}

#[derive(Value)]
pub struct InlineLogRecordAttributes<'a> {
    #[sval(index = 2)]
    pub severity_number: SeverityNumber,
    #[sval(index = 3)]
    pub severity_text: &'a str,
    #[sval(index = 6)]
    pub attributes: &'a [KeyValue<&'a str, &'a AnyValue<'a>>],
    #[sval(index = 9)]
    pub trace_id: &'a sval::BinaryArray<16>,
    #[sval(index = 10)]
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
            &sval::Label::new("attributes").with_tag(&sval::tags::VALUE_IDENT),
            &sval::Index::new(6),
        )?;
        stream.seq_begin(None)?;

        let mut seen = HashSet::new();
        self.0.for_each(|k, v| {
            match k.as_str() {
                emit_core::well_known::LVL_KEY => {
                    level = v.to_level().unwrap_or_default();
                }
                emit_core::well_known::SPAN_ID_KEY => {
                    span_id = v
                        .to_span_id()
                        .map(|span_id| span_id.to_hex())
                        .unwrap_or_default();
                }
                emit_core::well_known::TRACE_ID_KEY => {
                    trace_id = v
                        .to_trace_id()
                        .map(|trace_id| trace_id.to_hex())
                        .unwrap_or_default();
                }
                _ => {
                    if seen.insert(k.to_owned()) {
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
                }
            }

            ControlFlow::Continue(())
        });

        stream.seq_end()?;
        stream.record_tuple_value_end(
            None,
            &sval::Label::new("attributes").with_tag(&sval::tags::VALUE_IDENT),
            &sval::Index::new(6),
        )?;

        let severity_number = match level {
            emit_core::level::Level::Debug => SeverityNumber::Debug as i32,
            emit_core::level::Level::Info => SeverityNumber::Info as i32,
            emit_core::level::Level::Warn => SeverityNumber::Warn as i32,
            emit_core::level::Level::Error => SeverityNumber::Error as i32,
        };

        stream.record_tuple_value_begin(
            None,
            &sval::Label::new("severityNumber").with_tag(&sval::tags::VALUE_IDENT),
            &sval::Index::new(2),
        )?;
        stream.i32(severity_number)?;
        stream.record_tuple_value_end(
            None,
            &sval::Label::new("severityNumber").with_tag(&sval::tags::VALUE_IDENT),
            &sval::Index::new(2),
        )?;

        stream.record_tuple_value_begin(
            None,
            &sval::Label::new("severityText").with_tag(&sval::tags::VALUE_IDENT),
            &sval::Index::new(3),
        )?;
        sval::stream_display(&mut *stream, level)?;
        stream.record_tuple_value_end(
            None,
            &sval::Label::new("severityText").with_tag(&sval::tags::VALUE_IDENT),
            &sval::Index::new(3),
        )?;

        if trace_id != [0; 32] {
            stream.record_tuple_value_begin(
                None,
                &sval::Label::new("traceId").with_tag(&sval::tags::VALUE_IDENT),
                &sval::Index::new(9),
            )?;
            stream.binary_begin(Some(32))?;
            stream.binary_fragment_computed(&trace_id)?;
            stream.binary_end()?;
            stream.record_tuple_value_end(
                None,
                &sval::Label::new("traceId").with_tag(&sval::tags::VALUE_IDENT),
                &sval::Index::new(9),
            )?;
        }

        if span_id != [0; 16] {
            stream.record_tuple_value_begin(
                None,
                &sval::Label::new("spanId").with_tag(&sval::tags::VALUE_IDENT),
                &sval::Index::new(10),
            )?;
            stream.binary_begin(Some(16))?;
            stream.binary_fragment_computed(&span_id)?;
            stream.binary_end()?;
            stream.record_tuple_value_end(
                None,
                &sval::Label::new("spanId").with_tag(&sval::tags::VALUE_IDENT),
                &sval::Index::new(10),
            )?;
        }

        stream.record_tuple_end(None, None, None)
    }
}
