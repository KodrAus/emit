use std::{cell::RefCell, ops::ControlFlow};

use emit::{
    str::ToStr,
    value::ToValue,
    well_known::{
        KEY_ERR, KEY_LVL, KEY_SPAN_ID, KEY_SPAN_PARENT, KEY_TRACE_ID, LVL_DEBUG, LVL_ERROR,
        LVL_INFO, LVL_WARN,
    },
    Filter,
};
use opentelemetry::{
    global::{self, BoxedTracer, GlobalLoggerProvider},
    logs::{AnyValue, LogRecord, Logger, LoggerProvider, Severity},
    trace::{SpanContext, SpanId, Status, TraceContextExt, TraceId, Tracer},
    Context, ContextGuard, Key, KeyValue, Value,
};

pub fn emitter(name: &'static str) -> OpenTelemetryEmitter {
    OpenTelemetryEmitter::new(name)
}

pub fn ctxt<C: emit::Ctxt>(name: &'static str, ctxt: C) -> OpenTelemetryContext<C> {
    OpenTelemetryContext::new(name, ctxt)
}

pub fn shutdown() {
    global::shutdown_logger_provider();
    global::shutdown_tracer_provider();
}

thread_local! {
    static CTXT_STACK: RefCell<Vec<CtxtFrame>> = RefCell::new(Vec::new());
}

struct CtxtFrame {
    _guard: ContextGuard,
    open: bool,
}

fn push(guard: CtxtFrame) {
    CTXT_STACK.with(|stack| stack.borrow_mut().push(guard));
}

fn pop() -> Option<CtxtFrame> {
    CTXT_STACK.with(|stack| stack.borrow_mut().pop())
}

fn with_current(f: impl FnOnce(&mut CtxtFrame)) {
    CTXT_STACK.with(|stack| {
        if let Some(frame) = stack.borrow_mut().last_mut() {
            f(frame);
        }
    })
}

pub struct OpenTelemetryContext<C> {
    tracer: BoxedTracer,
    ctxt: C,
}

pub struct OpenTelemetryContextFrame<F> {
    ctxt: Option<Context>,
    attached: bool,
    open: bool,
    frame: F,
}

pub struct OpenTelemetryContextProps<P: ?Sized> {
    trace_id: Option<emit::span::TraceId>,
    span_id: Option<emit::span::SpanId>,
    props: *const P,
}

impl<C> OpenTelemetryContext<C> {
    pub fn new(name: &'static str, ctxt: C) -> Self {
        OpenTelemetryContext {
            tracer: global::tracer(name),
            ctxt,
        }
    }
}

impl<P: emit::Props + ?Sized> emit::Props for OpenTelemetryContextProps<P> {
    fn for_each<'kv, F: FnMut(emit::Str<'kv>, emit::Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        if let Some(ref trace_id) = self.trace_id {
            for_each(emit::well_known::KEY_TRACE_ID.to_str(), trace_id.to_value())?;
        }

        if let Some(ref span_id) = self.span_id {
            for_each(emit::well_known::KEY_SPAN_ID.to_str(), span_id.to_value())?;
        }

        unsafe { &*self.props }.for_each(|k, v| {
            if k != emit::well_known::KEY_TRACE_ID && k != emit::well_known::KEY_SPAN_ID {
                for_each(k, v)?;
            }

            ControlFlow::Continue(())
        })
    }
}

impl<C: emit::Ctxt> emit::Ctxt for OpenTelemetryContext<C> {
    type Current = OpenTelemetryContextProps<C::Current>;

    type Frame = OpenTelemetryContextFrame<C::Frame>;

    fn open_root<P: emit::Props>(&self, props: P) -> Self::Frame {
        let trace_id = props.pull::<emit::span::TraceId, _>(emit::well_known::KEY_TRACE_ID);
        let span_id = props.pull::<emit::span::SpanId, _>(emit::well_known::KEY_SPAN_ID);

        // Only open a span if the props include a span id
        if let Some(span_id) = span_id {
            let ctxt = Context::current();

            let span_id = otel_span_id(span_id);

            // Only open a span if the id has changed
            if span_id != ctxt.span().span_context().span_id() {
                let trace_id = trace_id.map(otel_trace_id);

                let mut span = self.tracer.span_builder("emit_span").with_span_id(span_id);

                if let Some(trace_id) = trace_id {
                    span = span.with_trace_id(trace_id);
                }

                let ctxt = ctxt.with_span(span.start(&self.tracer));

                return OpenTelemetryContextFrame {
                    attached: false,
                    open: true,
                    ctxt: Some(ctxt),
                    frame: self.ctxt.open_root(props),
                };
            }
        }

        OpenTelemetryContextFrame {
            attached: false,
            open: false,
            ctxt: None,
            frame: self.ctxt.open_root(props),
        }
    }

    fn enter(&self, local: &mut Self::Frame) {
        self.ctxt.enter(&mut local.frame);

        if let Some(ctxt) = local.ctxt.take() {
            let guard = ctxt.attach();

            push(CtxtFrame {
                _guard: guard,
                open: local.open,
            });
            local.attached = true;
        }
    }

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        let ctxt = Context::current();
        let span = ctxt.span();

        let trace_id = span.span_context().trace_id().to_bytes();
        let span_id = span.span_context().span_id().to_bytes();

        self.ctxt.with_current(|props| {
            let props = OpenTelemetryContextProps {
                trace_id: emit::span::TraceId::from_bytes(trace_id),
                span_id: emit::span::SpanId::from_bytes(span_id),
                props: props as *const C::Current,
            };

            with(&props)
        })
    }

    fn exit(&self, local: &mut Self::Frame) {
        self.ctxt.exit(&mut local.frame);

        if local.attached {
            local.ctxt = Some(Context::current());

            if let Some(frame) = pop() {
                local.open = frame.open;
            }
            local.attached = false;
        }
    }

    fn close(&self, mut local: Self::Frame) {
        // If the span hasn't been closed through an event, then close it now
        if local.open {
            if let Some(ctxt) = local.ctxt.take() {
                let span = ctxt.span();
                span.end();
            }
        }
    }
}

pub struct OpenTelemetryEmitter {
    logger: <GlobalLoggerProvider as LoggerProvider>::Logger,
}

impl OpenTelemetryEmitter {
    pub fn new(name: &'static str) -> Self {
        OpenTelemetryEmitter {
            logger: global::logger_provider().logger(name),
        }
    }
}

impl emit::Emitter for OpenTelemetryEmitter {
    fn emit<P: emit::Props>(&self, evt: &emit::Event<P>) {
        if emit::kind::is_span_filter().matches(evt) {
            let mut emitted = false;
            with_current(|frame| {
                if frame.open {
                    let ctxt = Context::current();
                    let span = ctxt.span();

                    let span_id = span.span_context().span_id();

                    let evt_span_id = evt
                        .props()
                        .pull::<emit::span::SpanId, _>(KEY_SPAN_ID)
                        .map(otel_span_id);

                    // If the event is for the current span then complete it
                    if Some(span_id) == evt_span_id {
                        if let Some(name) = evt
                            .props()
                            .pull::<emit::Str, _>(emit::well_known::KEY_SPAN_NAME)
                        {
                            span.update_name(name.to_cow());
                        } else {
                            span.update_name(evt.tpl().to_string())
                        }

                        evt.props().for_each(|k, v| {
                            if k == KEY_LVL {
                                if let Some(emit::Level::Error) = v.by_ref().cast::<emit::Level>() {
                                    span.set_status(Status::error("error"));

                                    return ControlFlow::Continue(());
                                }
                            }

                            if k == KEY_ERR {
                                span.add_event(
                                    "exception",
                                    vec![KeyValue::new("exception.message", v.to_string())],
                                );

                                return ControlFlow::Continue(());
                            }

                            if k == KEY_TRACE_ID || k == KEY_SPAN_ID || k == KEY_SPAN_PARENT {
                                return ControlFlow::Continue(());
                            }

                            if let Some(v) = otel_span_value(v) {
                                span.set_attribute(KeyValue::new(k.to_cow(), v));
                            }

                            ControlFlow::Continue(())
                        });

                        if let Some(extent) = evt.extent().and_then(|ex| ex.as_span()) {
                            span.end_with_timestamp(extent.end.to_system_time());
                        } else {
                            span.end();
                        }

                        frame.open = false;
                        emitted = true;
                    }
                }
            });

            if emitted {
                return;
            }
        }

        let mut record = LogRecord::builder().with_body(evt.msg().to_string());

        let mut trace_id = None;
        let mut span_id = None;
        let mut attributes = Vec::new();
        {
            let mut slot = Some(record);
            evt.props().for_each(|k, v| {
                if k == KEY_LVL {
                    match v.by_ref().cast::<emit::Level>() {
                        Some(emit::Level::Debug) => {
                            slot = Some(
                                slot.take()
                                    .unwrap()
                                    .with_severity_number(Severity::Debug)
                                    .with_severity_text(LVL_DEBUG),
                            );
                        }
                        Some(emit::Level::Info) => {
                            slot = Some(
                                slot.take()
                                    .unwrap()
                                    .with_severity_number(Severity::Info)
                                    .with_severity_text(LVL_INFO),
                            );
                        }
                        Some(emit::Level::Warn) => {
                            slot = Some(
                                slot.take()
                                    .unwrap()
                                    .with_severity_number(Severity::Warn)
                                    .with_severity_text(LVL_WARN),
                            );
                        }
                        Some(emit::Level::Error) => {
                            slot = Some(
                                slot.take()
                                    .unwrap()
                                    .with_severity_number(Severity::Error)
                                    .with_severity_text(LVL_ERROR),
                            );
                        }
                        None => {
                            slot = Some(slot.take().unwrap().with_severity_text(v.to_string()));
                        }
                    }

                    return ControlFlow::Continue(());
                }

                if k == KEY_TRACE_ID {
                    if let Some(id) = v.by_ref().cast::<emit::span::TraceId>() {
                        trace_id = Some(otel_trace_id(id));

                        return ControlFlow::Continue(());
                    }
                }

                if k == KEY_SPAN_ID {
                    if let Some(id) = v.by_ref().cast::<emit::span::SpanId>() {
                        span_id = Some(otel_span_id(id));

                        return ControlFlow::Continue(());
                    }
                }

                if k == KEY_SPAN_PARENT {
                    if v.by_ref().cast::<emit::span::SpanId>().is_some() {
                        return ControlFlow::Continue(());
                    }
                }

                if let Some(v) = otel_log_value(v) {
                    attributes.push((Key::new(k.to_cow()), v));
                }

                ControlFlow::Continue(())
            });

            record = slot.unwrap();
        }

        record = record.with_attributes(attributes);

        if let (Some(trace_id), Some(span_id)) = (trace_id, span_id) {
            let ctxt = Context::current();
            let span = ctxt.span();

            let ctxt = span.span_context();

            record = record.with_span_context(&SpanContext::new(
                trace_id,
                span_id,
                ctxt.trace_flags(),
                ctxt.is_remote(),
                ctxt.trace_state().clone(),
            ))
        }

        if let Some(extent) = evt.extent() {
            record = record.with_timestamp(extent.as_point().to_system_time());
        }

        self.logger.emit(record.build());
    }

    fn blocking_flush(&self, _: std::time::Duration) {}
}

fn otel_trace_id(trace_id: emit::span::TraceId) -> TraceId {
    TraceId::from_bytes(trace_id.to_bytes())
}

fn otel_span_id(span_id: emit::span::SpanId) -> SpanId {
    SpanId::from_bytes(span_id.to_bytes())
}

fn otel_span_value(v: emit::Value) -> Option<Value> {
    match any_value::serialize(&v) {
        Ok(Some(av)) => match av {
            AnyValue::Int(v) => Some(Value::I64(v)),
            AnyValue::Double(v) => Some(Value::F64(v)),
            AnyValue::String(v) => Some(Value::String(v)),
            AnyValue::Boolean(v) => Some(Value::Bool(v)),
            // Variants not supported by `Value`
            AnyValue::Bytes(_) => Some(Value::String(v.to_string().into())),
            AnyValue::ListAny(_) => Some(Value::String(v.to_string().into())),
            AnyValue::Map(_) => Some(Value::String(v.to_string().into())),
        },
        Ok(None) => None,
        Err(()) => Some(Value::String(v.to_string().into())),
    }
}

fn otel_log_value(v: emit::Value) -> Option<AnyValue> {
    match any_value::serialize(&v) {
        Ok(v) => v,
        Err(()) => Some(AnyValue::String(v.to_string().into())),
    }
}

mod any_value {
    use std::{collections::HashMap, fmt};

    use opentelemetry::{logs::AnyValue, Key, StringValue};
    use serde::ser::{
        Error, Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
        SerializeTuple, SerializeTupleStruct, SerializeTupleVariant, Serializer, StdError,
    };

    /// Serialize an arbitrary `serde::Serialize` into an `AnyValue`.
    ///
    /// This method performs the following translations when converting between `serde`'s data model and OpenTelemetry's:
    ///
    /// - Integers that don't fit in a `i64` are converted into strings.
    /// - Unit types and nones are discarded (effectively treated as undefined).
    /// - Struct and tuple variants are converted into an internally tagged map.
    /// - Unit variants are converted into strings.
    pub(crate) fn serialize(value: impl serde::Serialize) -> Result<Option<AnyValue>, ()> {
        value.serialize(ValueSerializer).map_err(|_| ())
    }

    struct ValueSerializer;

    struct ValueSerializeSeq {
        value: Vec<AnyValue>,
    }

    struct ValueSerializeTuple {
        value: Vec<AnyValue>,
    }

    struct ValueSerializeTupleStruct {
        value: Vec<AnyValue>,
    }

    struct ValueSerializeMap {
        key: Option<Key>,
        value: HashMap<Key, AnyValue>,
    }

    struct ValueSerializeStruct {
        value: HashMap<Key, AnyValue>,
    }

    struct ValueSerializeTupleVariant {
        variant: &'static str,
        value: Vec<AnyValue>,
    }

    struct ValueSerializeStructVariant {
        variant: &'static str,
        value: HashMap<Key, AnyValue>,
    }

    #[derive(Debug)]
    struct ValueError(String);

    impl fmt::Display for ValueError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Display::fmt(&self.0, f)
        }
    }

    impl Error for ValueError {
        fn custom<T>(msg: T) -> Self
        where
            T: fmt::Display,
        {
            ValueError(msg.to_string())
        }
    }

    impl StdError for ValueError {}

    impl Serializer for ValueSerializer {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        type SerializeSeq = ValueSerializeSeq;

        type SerializeTuple = ValueSerializeTuple;

        type SerializeTupleStruct = ValueSerializeTupleStruct;

        type SerializeTupleVariant = ValueSerializeTupleVariant;

        type SerializeMap = ValueSerializeMap;

        type SerializeStruct = ValueSerializeStruct;

        type SerializeStructVariant = ValueSerializeStructVariant;

        fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Boolean(v)))
        }

        fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(v as i64)
        }

        fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(v as i64)
        }

        fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(v as i64)
        }

        fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Int(v)))
        }

        fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
            if let Ok(v) = v.try_into() {
                self.serialize_i64(v)
            } else {
                self.collect_str(&v)
            }
        }

        fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(v as i64)
        }

        fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(v as i64)
        }

        fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(v as i64)
        }

        fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
            if let Ok(v) = v.try_into() {
                self.serialize_i64(v)
            } else {
                self.collect_str(&v)
            }
        }

        fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
            if let Ok(v) = v.try_into() {
                self.serialize_i64(v)
            } else {
                self.collect_str(&v)
            }
        }

        fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
            self.serialize_f64(v as f64)
        }

        fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Double(v)))
        }

        fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
            self.collect_str(&v)
        }

        fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::String(StringValue::from(v.to_owned()))))
        }

        fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Bytes(v.to_owned())))
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Ok(None)
        }

        fn serialize_some<T: serde::Serialize + ?Sized>(
            self,
            value: &T,
        ) -> Result<Self::Ok, Self::Error> {
            value.serialize(self)
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Ok(None)
        }

        fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
            name.serialize(self)
        }

        fn serialize_unit_variant(
            self,
            _: &'static str,
            _: u32,
            variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            variant.serialize(self)
        }

        fn serialize_newtype_struct<T: serde::Serialize + ?Sized>(
            self,
            _: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error> {
            value.serialize(self)
        }

        fn serialize_newtype_variant<T: serde::Serialize + ?Sized>(
            self,
            _: &'static str,
            _: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error> {
            let mut map = self.serialize_map(Some(1))?;
            map.serialize_entry(variant, value)?;
            map.end()
        }

        fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Ok(ValueSerializeSeq { value: Vec::new() })
        }

        fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Ok(ValueSerializeTuple { value: Vec::new() })
        }

        fn serialize_tuple_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Ok(ValueSerializeTupleStruct { value: Vec::new() })
        }

        fn serialize_tuple_variant(
            self,
            _: &'static str,
            _: u32,
            variant: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Ok(ValueSerializeTupleVariant {
                variant,
                value: Vec::new(),
            })
        }

        fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Ok(ValueSerializeMap {
                key: None,
                value: HashMap::new(),
            })
        }

        fn serialize_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            Ok(ValueSerializeStruct {
                value: HashMap::new(),
            })
        }

        fn serialize_struct_variant(
            self,
            _: &'static str,
            _: u32,
            variant: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Ok(ValueSerializeStructVariant {
                variant,
                value: HashMap::new(),
            })
        }
    }

    impl SerializeSeq for ValueSerializeSeq {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_element<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.push(value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::ListAny(self.value)))
        }
    }

    impl SerializeTuple for ValueSerializeTuple {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_element<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.push(value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::ListAny(self.value)))
        }
    }

    impl SerializeTupleStruct for ValueSerializeTupleStruct {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_field<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.push(value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::ListAny(self.value)))
        }
    }

    impl SerializeTupleVariant for ValueSerializeTupleVariant {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_field<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.push(value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Map({
                let mut variant = HashMap::new();
                variant.insert(Key::from(self.variant), AnyValue::ListAny(self.value));
                variant
            })))
        }
    }

    impl SerializeMap for ValueSerializeMap {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_key<T: serde::Serialize + ?Sized>(
            &mut self,
            key: &T,
        ) -> Result<(), Self::Error> {
            let key = match key.serialize(ValueSerializer)? {
                Some(AnyValue::String(key)) => Key::from(String::from(key)),
                key => Key::from(format!("{:?}", key)),
            };

            self.key = Some(key);

            Ok(())
        }

        fn serialize_value<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            let key = self
                .key
                .take()
                .ok_or_else(|| Self::Error::custom("missing key"))?;

            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.insert(key, value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Map(self.value)))
        }
    }

    impl SerializeStruct for ValueSerializeStruct {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_field<T: serde::Serialize + ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            let key = Key::from(key);

            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.insert(key, value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Map(self.value)))
        }
    }

    impl SerializeStructVariant for ValueSerializeStructVariant {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_field<T: serde::Serialize + ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            let key = Key::from(key);

            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.insert(key, value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Map({
                let mut variant = HashMap::new();
                variant.insert(Key::from(self.variant), AnyValue::Map(self.value));
                variant
            })))
        }
    }
}
