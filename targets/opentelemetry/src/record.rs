use std::{fmt, ops::ControlFlow, str, time::SystemTime};

use opentelemetry_api::{
    logs::{AnyValue, LogRecord, Severity, TraceContext},
    trace::{SpanContext, SpanId, TraceId},
    Key, OrderMap,
};
use serde::ser::{
    Error, Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant, Serializer, StdError,
};

pub(crate) fn to_value(value: emit::Value) -> Option<AnyValue> {
    value.serialize(ValueSerializer).ok()
}

// TODO: `Null`s should probably just exclude the attribute altogether
pub(crate) fn to_record(evt: &emit::Event<impl emit::Props>) -> LogRecord {
    let mut builder = LogRecord::builder();

    if let (Some(trace), Some(span)) = (evt.id().trace(), evt.id().span()) {
        let trace_id = trace.to_hex();
        let span_id = span.to_hex();

        builder = builder.with_span_context(&SpanContext::new(
            TraceId::from_hex(str::from_utf8(&trace_id).unwrap_or("")).unwrap_or(TraceId::INVALID),
            SpanId::from_hex(str::from_utf8(&span_id).unwrap_or("")).unwrap_or(SpanId::INVALID),
            Default::default(),
            Default::default(),
            Default::default(),
        ));
    }

    LogRecord::builder()
        .with_timestamp(
            evt.ts()
                .map(|ts| ts.to_system_time())
                .unwrap_or_else(SystemTime::now),
        )
        .with_severity_number(match evt.lvl() {
            emit::Level::Debug => Severity::Debug,
            emit::Level::Info => Severity::Info,
            emit::Level::Warn => Severity::Warn,
            emit::Level::Error => Severity::Error,
        })
        .with_severity_text(evt.lvl().to_string())
        .with_body(AnyValue::String(evt.msg().to_string().into()))
        .with_attributes({
            let mut attributes = OrderMap::<Key, AnyValue>::new();

            evt.props().for_each(|k, v| {
                if let Some(value) = to_value(v) {
                    let key = k.to_string().into();

                    attributes.insert(key, value);
                }

                ControlFlow::Continue(())
            });

            attributes
        })
        .build()
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
    value: OrderMap<Key, AnyValue>,
}

struct ValueSerializeStruct {
    value: OrderMap<Key, AnyValue>,
}

struct ValueSerializeTupleVariant {
    variant: &'static str,
    value: Vec<AnyValue>,
}

struct ValueSerializeStructVariant {
    variant: &'static str,
    value: OrderMap<Key, AnyValue>,
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
    type Ok = AnyValue;

    type Error = ValueError;

    type SerializeSeq = ValueSerializeSeq;

    type SerializeTuple = ValueSerializeTuple;

    type SerializeTupleStruct = ValueSerializeTupleStruct;

    type SerializeTupleVariant = ValueSerializeTupleVariant;

    type SerializeMap = ValueSerializeMap;

    type SerializeStruct = ValueSerializeStruct;

    type SerializeStructVariant = ValueSerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::Boolean(v))
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
        Ok(AnyValue::Int(v))
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
        Ok(AnyValue::Double(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.collect_str(&v)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::String(v.to_owned().into()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::Bytes(v.to_owned()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::ListAny(vec![]))
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::ListAny(vec![]))
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

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
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
            value: OrderMap::new(),
        })
    }

    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(ValueSerializeStruct {
            value: OrderMap::new(),
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
            value: OrderMap::new(),
        })
    }
}

impl SerializeSeq for ValueSerializeSeq {
    type Ok = AnyValue;

    type Error = ValueError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        Ok(self.value.push(value.serialize(ValueSerializer)?))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::ListAny(self.value))
    }
}

impl SerializeTuple for ValueSerializeTuple {
    type Ok = AnyValue;

    type Error = ValueError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        Ok(self.value.push(value.serialize(ValueSerializer)?))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::ListAny(self.value))
    }
}

impl SerializeTupleStruct for ValueSerializeTupleStruct {
    type Ok = AnyValue;

    type Error = ValueError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        Ok(self.value.push(value.serialize(ValueSerializer)?))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::ListAny(self.value))
    }
}

impl SerializeTupleVariant for ValueSerializeTupleVariant {
    type Ok = AnyValue;

    type Error = ValueError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.value.push(value.serialize(ValueSerializer)?);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut wrapper = OrderMap::new();
        wrapper.insert(
            Key::from_static_str(self.variant),
            AnyValue::ListAny(self.value),
        );

        Ok(AnyValue::Map(wrapper))
    }
}

impl SerializeMap for ValueSerializeMap {
    type Ok = AnyValue;

    type Error = ValueError;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let key = match key.serialize(ValueSerializer)? {
            AnyValue::String(key) => Key::from(key.as_str().to_owned()),
            key => Key::from(format!("{:?}", key)),
        };

        // Use a dummy value; a valid serializer will replace it shortly
        self.value.insert(key, AnyValue::ListAny(vec![]));

        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        *self
            .value
            .last_mut()
            .ok_or_else(|| Error::custom("missing key"))?
            .1 = value.serialize(ValueSerializer)?;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::Map(self.value))
    }
}

impl SerializeStruct for ValueSerializeStruct {
    type Ok = AnyValue;

    type Error = ValueError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.value
            .insert(Key::from_static_str(key), value.serialize(ValueSerializer)?);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::Map(self.value))
    }
}

impl SerializeStructVariant for ValueSerializeStructVariant {
    type Ok = AnyValue;

    type Error = ValueError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.value
            .insert(Key::from_static_str(key), value.serialize(ValueSerializer)?);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut wrapper = OrderMap::new();
        wrapper.insert(
            Key::from_static_str(self.variant),
            AnyValue::Map(self.value),
        );

        Ok(AnyValue::Map(wrapper))
    }
}
