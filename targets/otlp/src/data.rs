use std::{collections::HashSet, ops::ControlFlow};
use sval_derive::Value;
use sval_protobuf::buf::ProtoBuf;

#[derive(Value)]
#[sval(dynamic)]
pub(crate) enum PreEncoded {
    Proto(ProtoBuf),
}

#[derive(Value)]
pub struct ExportLogsServiceRequest<'a, RL> {
    #[sval(index = 1)]
    pub resource_logs: &'a [RL],
}

#[derive(Value)]
pub struct ResourceLogs<'a, R, SL> {
    #[sval(index = 1)]
    pub resource: Option<R>,
    #[sval(index = 2)]
    pub scope_logs: &'a [SL],
    #[sval(index = 3)]
    pub schema_url: &'a str,
}

#[derive(Value)]
pub struct Resource<'a> {
    #[sval(index = 1)]
    pub attributes: &'a [KeyValue<'a>],
    #[sval(index = 2)]
    pub dropped_attribute_count: u32,
}

#[derive(Value)]
pub struct ScopeLogs<'a, IS, LR> {
    #[sval(index = 1)]
    pub scope: Option<IS>,
    #[sval(index = 2)]
    pub log_records: &'a [LR],
    #[sval(index = 3)]
    pub schema_url: &'a str,
}

#[derive(Value)]
pub struct InstrumentationScope<'a> {
    #[sval(index = 1)]
    pub name: &'a str,
    #[sval(index = 2)]
    pub version: &'a str,
    #[sval(index = 3)]
    pub attributes: &'a [KeyValue<'a>],
    #[sval(index = 4)]
    pub dropped_attribute_count: u32,
}

#[derive(Value)]
#[repr(i32)]
pub enum SeverityNumber {
    Unspecified = 0,
    Trace = 1,
    Debug = 5,
    Info = 9,
    Warn = 13,
    Error = 17,
    Fatal = 21,
}

const ANY_VALUE_STRING_LABEL: sval::Label =
    sval::Label::new("stringValue").with_tag(&sval::tags::VALUE_IDENT);
const ANY_VALUE_BOOL_LABEL: sval::Label =
    sval::Label::new("boolValue").with_tag(&sval::tags::VALUE_IDENT);
const ANY_VALUE_INT_LABEL: sval::Label =
    sval::Label::new("intValue").with_tag(&sval::tags::VALUE_IDENT);
const ANY_VALUE_DOUBLE_LABEL: sval::Label =
    sval::Label::new("doubleValue").with_tag(&sval::tags::VALUE_IDENT);
const ANY_VALUE_ARRAY_LABEL: sval::Label =
    sval::Label::new("arrayValue").with_tag(&sval::tags::VALUE_IDENT);
const ANY_VALUE_KVLIST_LABEL: sval::Label =
    sval::Label::new("kvlistValue").with_tag(&sval::tags::VALUE_IDENT);
const ANY_VALUE_BYTES_LABEL: sval::Label =
    sval::Label::new("bytesValue").with_tag(&sval::tags::VALUE_IDENT);

const ANY_VALUE_STRING_INDEX: sval::Index = sval::Index::new(1);
const ANY_VALUE_BOOL_INDEX: sval::Index = sval::Index::new(2);
const ANY_VALUE_INT_INDEX: sval::Index = sval::Index::new(3);
const ANY_VALUE_DOUBLE_INDEX: sval::Index = sval::Index::new(4);
const ANY_VALUE_ARRAY_INDEX: sval::Index = sval::Index::new(5);
const ANY_VALUE_KVLIST_INDEX: sval::Index = sval::Index::new(6);
const ANY_VALUE_BYTES_INDEX: sval::Index = sval::Index::new(7);

// TODO: Use the consts here
#[derive(Value)]
pub enum AnyValue<'a> {
    #[sval(index = 1)]
    String(&'a str),
    #[sval(index = 2)]
    Bool(bool),
    #[sval(index = 3)]
    Int(i64),
    #[sval(index = 4)]
    Double(f64),
    #[sval(index = 5)]
    Array(ArrayValue<'a>),
    #[sval(index = 6)]
    Kvlist(KvList<'a>),
    #[sval(index = 7)]
    Bytes(&'a sval::BinarySlice),
}

#[derive(Value)]
pub struct ArrayValue<'a> {
    #[sval(index = 1)]
    pub values: &'a [AnyValue<'a>],
}

#[derive(Value)]
pub struct KvList<'a> {
    #[sval(index = 1)]
    pub values: &'a [KeyValue<'a>],
}

#[derive(Value)]
pub struct KeyValue<'a> {
    #[sval(index = 1)]
    pub key: &'a str,
    #[sval(index = 2)]
    pub value: Option<AnyValue<'a>>,
}

#[derive(Value)]
pub struct LogRecord<B, A> {
    #[sval(index = 1, data_tag = "sval_protobuf::tags::PROTOBUF_I64")]
    pub time_unix_nano: u64,
    #[sval(index = 11, data_tag = "sval_protobuf::tags::PROTOBUF_I64")]
    pub observed_time_unix_nano: u64,
    #[sval(index = 7)]
    pub dropped_attributes_count: u32,
    #[sval(index = 8, data_tag = "sval_protobuf::tags::PROTOBUF_I32")]
    pub flags: u32,
    #[sval(index = 5)]
    pub body: Option<B>,
    #[sval(flatten)]
    pub attributes: A,
}

#[derive(Value)]
pub struct InlineAttributes<'a> {
    #[sval(index = 2)]
    pub severity_number: SeverityNumber,
    #[sval(index = 3)]
    pub severity_text: &'a str,
    #[sval(index = 6)]
    pub attributes: &'a [KeyValue<'a>],
    #[sval(index = 9)]
    pub trace_id: &'a sval::BinaryArray<16>,
    #[sval(index = 10)]
    pub span_id: &'a sval::BinaryArray<8>,
}

pub struct PropsAttributes<P>(pub P);

impl<P: emit_core::props::Props> sval::Value for PropsAttributes<P> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        let mut trace_id = [0; 32];
        let mut span_id = [0; 16];
        let mut level = emit_core::level::Level::default();

        stream.tuple_begin(None, None, None, None)?;

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
                            .tuple_value_begin(None, &sval::Index::new(6))
                            .map(|_| ControlFlow::Continue(()))
                            .unwrap_or(ControlFlow::Break(()))?;
                        sval_ref::stream_ref(&mut *stream, EmitValue(v))
                            .map(|_| ControlFlow::Continue(()))
                            .unwrap_or(ControlFlow::Break(()))?;
                        stream
                            .tuple_value_end(None, &sval::Index::new(6))
                            .map(|_| ControlFlow::Continue(()))
                            .unwrap_or(ControlFlow::Break(()))?;
                    }
                }
            }

            ControlFlow::Continue(())
        });

        let severity_number = match level {
            emit_core::level::Level::Debug => 1i32,
            emit_core::level::Level::Info => 2i32,
            emit_core::level::Level::Warn => 3i32,
            emit_core::level::Level::Error => 4i32,
        };

        stream.tuple_value_begin(None, &sval::Index::new(2))?;
        stream.i32(severity_number)?;
        stream.tuple_value_end(None, &sval::Index::new(2))?;

        stream.tuple_value_begin(None, &sval::Index::new(3))?;
        sval::stream_display(&mut *stream, level)?;
        stream.tuple_value_end(None, &sval::Index::new(3))?;

        if trace_id != [0; 32] {
            stream.tuple_value_begin(None, &sval::Index::new(9))?;
            stream.binary_begin(Some(32))?;
            stream.binary_fragment_computed(&trace_id)?;
            stream.binary_end()?;
            stream.tuple_value_end(None, &sval::Index::new(9))?;
        }

        if span_id != [0; 16] {
            stream.tuple_value_begin(None, &sval::Index::new(10))?;
            stream.binary_begin(Some(16))?;
            stream.binary_fragment_computed(&span_id)?;
            stream.binary_end()?;
            stream.tuple_value_end(None, &sval::Index::new(10))?;
        }

        stream.tuple_end(None, None, None)
    }
}

#[derive(Value)]
pub enum DisplayValue<D> {
    #[sval(index = 1)]
    String(D),
}

#[repr(transparent)]
struct EmitValue<'a>(pub emit_core::value::Value<'a>);

impl<'a> sval::Value for EmitValue<'a> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        use sval_ref::ValueRef as _;

        self.stream_ref(stream)
    }
}

impl<'a> sval_ref::ValueRef<'a> for EmitValue<'a> {
    fn stream_ref<S: sval::Stream<'a> + ?Sized>(&self, stream: &mut S) -> sval::Result {
        struct AnyStream<S> {
            stream: S,
            in_map_key: bool,
        }

        impl<'sval, S: sval::Stream<'sval>> AnyStream<S> {
            fn any_value_begin(
                &mut self,
                label: &sval::Label,
                index: &sval::Index,
            ) -> sval::Result {
                self.stream.enum_begin(None, None, None)?;
                self.stream.tagged_begin(None, Some(label), Some(index))
            }

            fn any_value_end(&mut self, label: &sval::Label, index: &sval::Index) -> sval::Result {
                self.stream.tagged_end(None, Some(label), Some(index))?;
                self.stream.enum_end(None, None, None)
            }
        }

        impl<'sval, S: sval::Stream<'sval>> sval::Stream<'sval> for AnyStream<S> {
            fn null(&mut self) -> sval::Result {
                self.stream.null()
            }

            fn bool(&mut self, value: bool) -> sval::Result {
                self.any_value_begin(&ANY_VALUE_BOOL_LABEL, &ANY_VALUE_BOOL_INDEX)?;
                self.stream.bool(value)?;
                self.any_value_end(&ANY_VALUE_BOOL_LABEL, &ANY_VALUE_BOOL_INDEX)
            }

            fn text_begin(&mut self, num_bytes: Option<usize>) -> sval::Result {
                self.any_value_begin(&ANY_VALUE_STRING_LABEL, &ANY_VALUE_STRING_INDEX)?;
                self.stream.text_begin(num_bytes)
            }

            fn text_fragment(&mut self, fragment: &'sval str) -> sval::Result {
                self.stream.text_fragment(fragment)
            }

            fn text_fragment_computed(&mut self, fragment: &str) -> sval::Result {
                self.stream.text_fragment_computed(fragment)
            }

            fn text_end(&mut self) -> sval::Result {
                self.stream.text_end()?;
                self.any_value_end(&ANY_VALUE_STRING_LABEL, &ANY_VALUE_STRING_INDEX)
            }

            fn i64(&mut self, value: i64) -> sval::Result {
                self.any_value_begin(&ANY_VALUE_INT_LABEL, &ANY_VALUE_INT_INDEX)?;
                self.stream.i64(value)?;
                self.any_value_end(&ANY_VALUE_INT_LABEL, &ANY_VALUE_INT_INDEX)
            }

            fn f64(&mut self, value: f64) -> sval::Result {
                self.any_value_begin(&ANY_VALUE_DOUBLE_LABEL, &ANY_VALUE_DOUBLE_INDEX)?;
                self.stream.f64(value)?;
                self.any_value_end(&ANY_VALUE_DOUBLE_LABEL, &ANY_VALUE_DOUBLE_INDEX)
            }

            fn binary_begin(&mut self, num_bytes: Option<usize>) -> sval::Result {
                self.any_value_begin(&ANY_VALUE_BYTES_LABEL, &ANY_VALUE_BYTES_INDEX)?;
                self.stream.text_begin(num_bytes)
            }

            fn binary_fragment(&mut self, fragment: &'sval [u8]) -> sval::Result {
                self.stream.binary_fragment(fragment)
            }

            fn binary_fragment_computed(&mut self, fragment: &[u8]) -> sval::Result {
                self.stream.binary_fragment_computed(fragment)
            }

            fn binary_end(&mut self) -> sval::Result {
                self.stream.text_end()?;
                self.any_value_end(&ANY_VALUE_BYTES_LABEL, &ANY_VALUE_BYTES_INDEX)
            }

            fn seq_begin(&mut self, num_entries: Option<usize>) -> sval::Result {
                self.any_value_begin(&ANY_VALUE_ARRAY_LABEL, &ANY_VALUE_ARRAY_INDEX)?;
                self.stream.record_tuple_begin(None, None, None, Some(1))?;
                self.stream.record_tuple_value_begin(
                    None,
                    &sval::Label::new("values").with_tag(&sval::tags::VALUE_IDENT),
                    &sval::Index::new(1),
                )?;
                self.stream.seq_begin(num_entries)
            }

            fn seq_value_begin(&mut self) -> sval::Result {
                self.stream.seq_value_begin()
            }

            fn seq_value_end(&mut self) -> sval::Result {
                self.stream.seq_value_end()
            }

            fn seq_end(&mut self) -> sval::Result {
                self.stream.seq_end()?;
                self.stream.record_tuple_value_end(
                    None,
                    &sval::Label::new("values").with_tag(&sval::tags::VALUE_IDENT),
                    &sval::Index::new(1),
                )?;
                self.stream.record_tuple_end(None, None, None)?;
                self.any_value_end(&ANY_VALUE_ARRAY_LABEL, &ANY_VALUE_ARRAY_INDEX)
            }

            fn map_begin(&mut self, num_entries: Option<usize>) -> sval::Result {
                self.any_value_begin(&ANY_VALUE_KVLIST_LABEL, &ANY_VALUE_KVLIST_INDEX)?;
                self.stream.record_tuple_begin(None, None, None, Some(1))?;
                self.stream.record_tuple_value_begin(
                    None,
                    &sval::Label::new("values").with_tag(&sval::tags::VALUE_IDENT),
                    &sval::Index::new(1),
                )?;
                self.stream.seq_begin(num_entries)
            }

            fn map_key_begin(&mut self) -> sval::Result {
                self.in_map_key = true;

                self.stream.seq_value_begin()?;
                self.stream.record_tuple_begin(None, None, None, Some(2))?;
                self.stream.record_tuple_value_begin(
                    None,
                    &sval::Label::new("key").with_tag(&sval::tags::VALUE_IDENT),
                    &sval::Index::new(1),
                )
            }

            fn map_key_end(&mut self) -> sval::Result {
                self.in_map_key = false;

                self.stream.record_tuple_value_end(
                    None,
                    &sval::Label::new("key").with_tag(&sval::tags::VALUE_IDENT),
                    &sval::Index::new(1),
                )
            }

            fn map_value_begin(&mut self) -> sval::Result {
                self.stream.record_tuple_value_begin(
                    None,
                    &sval::Label::new("value").with_tag(&sval::tags::VALUE_IDENT),
                    &sval::Index::new(2),
                )
            }

            fn map_value_end(&mut self) -> sval::Result {
                self.stream.record_tuple_value_end(
                    None,
                    &sval::Label::new("value").with_tag(&sval::tags::VALUE_IDENT),
                    &sval::Index::new(2),
                )?;
                self.stream.record_tuple_end(None, None, None)?;
                self.stream.seq_value_end()
            }

            fn map_end(&mut self) -> sval::Result {
                self.stream.seq_end()?;
                self.stream.record_tuple_value_end(
                    None,
                    &sval::Label::new("values").with_tag(&sval::tags::VALUE_IDENT),
                    &sval::Index::new(1),
                )?;
                self.stream.record_tuple_end(None, None, None)?;
                self.any_value_end(&ANY_VALUE_KVLIST_LABEL, &ANY_VALUE_KVLIST_INDEX)
            }
        }

        sval_ref::stream_ref(
            &mut AnyStream {
                stream,
                in_map_key: false,
            },
            &self.0,
        )
    }
}
