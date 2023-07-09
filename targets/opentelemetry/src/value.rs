use std::{fmt, str};

use opentelemetry_api::{logs::AnyValue, Key, OrderMap, Value};
use serde::ser::{
    Error, Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant, Serializer, StdError,
};

pub(crate) fn to_any_value(value: emit_core::value::Value) -> Option<AnyValue> {
    value.serialize(ValueSerializer).unwrap_or(None)
}

pub(crate) fn to_value(value: emit_core::value::Value) -> Value {
    value
        .by_ref()
        .serialize(ValueSerializer)
        .ok()
        .flatten()
        .and_then(|v| match v {
            AnyValue::Int(v) => Some(Value::I64(v)),
            AnyValue::Double(v) => Some(Value::F64(v)),
            AnyValue::String(v) => Some(Value::String(v)),
            AnyValue::Boolean(v) => Some(Value::Bool(v)),
            _ => None,
        })
        .unwrap_or_else(|| Value::from(value.to_string()))
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
        Ok(Some(AnyValue::String(v.to_owned().into())))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Some(AnyValue::Bytes(v.to_owned())))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(None)
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
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

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
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
        T: Serialize,
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
            key: None,
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
    type Ok = Option<AnyValue>;

    type Error = ValueError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
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

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
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

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
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

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        if let Some(value) = value.serialize(ValueSerializer)? {
            self.value.push(value);
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut wrapper = OrderMap::new();
        wrapper.insert(
            Key::from_static_str(self.variant),
            AnyValue::ListAny(self.value),
        );

        Ok(Some(AnyValue::Map(wrapper)))
    }
}

impl SerializeMap for ValueSerializeMap {
    type Ok = Option<AnyValue>;

    type Error = ValueError;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let key = match key.serialize(ValueSerializer)? {
            Some(AnyValue::String(key)) => Key::from(key.as_str().to_owned()),
            key => Key::from(format!("{:?}", key)),
        };

        self.key = Some(key);

        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        if let (Some(key), Some(value)) = (self.key.take(), value.serialize(ValueSerializer)?) {
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

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        if let Some(value) = value.serialize(ValueSerializer)? {
            self.value.insert(Key::from_static_str(key), value);
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

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        if let Some(value) = value.serialize(ValueSerializer)? {
            self.value.insert(Key::from_static_str(key), value);
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut wrapper = OrderMap::new();
        wrapper.insert(
            Key::from_static_str(self.variant),
            AnyValue::Map(self.value),
        );

        Ok(Some(AnyValue::Map(wrapper)))
    }
}
