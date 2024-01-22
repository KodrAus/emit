#[path = ""]
pub(crate) mod logs {
    #[path = "./generated/opentelemetry.proto.logs.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod trace {
    #[path = "./generated/opentelemetry.proto.trace.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod metrics {
    #[path = "./generated/opentelemetry.proto.metrics.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod common {
    #[path = "./generated/opentelemetry.proto.common.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod resource {
    #[path = "./generated/opentelemetry.proto.resource.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod collector {
    #[path = ""]
    pub(crate) mod logs {
        #[path = "./generated/opentelemetry.proto.collector.logs.v1.rs"]
        pub(crate) mod v1;
    }

    #[path = ""]
    pub(crate) mod trace {
        #[path = "./generated/opentelemetry.proto.collector.trace.v1.rs"]
        pub(crate) mod v1;
    }

    #[path = ""]
    pub(crate) mod metrics {
        #[path = "./generated/opentelemetry.proto.collector.metrics.v1.rs"]
        pub(crate) mod v1;
    }
}

#[cfg(feature = "grpc")]
pub(crate) mod any_value {
    use std::fmt;

    use serde::ser::{
        Error, Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
        SerializeTuple, SerializeTupleStruct, SerializeTupleVariant, Serializer, StdError,
    };

    use super::common::v1::{any_value::Value, AnyValue, ArrayValue, KeyValue, KeyValueList};

    pub(crate) fn to_value(value: emit::value::Value) -> Option<AnyValue> {
        value.serialize(ValueSerializer).ok()
    }

    struct ValueSerializer;

    struct ValueSerializeSeq {
        value: ArrayValue,
    }

    struct ValueSerializeTuple {
        value: ArrayValue,
    }

    struct ValueSerializeTupleStruct {
        value: ArrayValue,
    }

    struct ValueSerializeMap {
        value: KeyValueList,
    }

    struct ValueSerializeStruct {
        value: KeyValueList,
    }

    struct ValueSerializeTupleVariant {
        variant: &'static str,
        value: ArrayValue,
    }

    struct ValueSerializeStructVariant {
        variant: &'static str,
        value: KeyValueList,
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
            Ok(AnyValue {
                value: Some(Value::BoolValue(v)),
            })
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
            Ok(AnyValue {
                value: Some(Value::IntValue(v)),
            })
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
            Ok(AnyValue {
                value: Some(Value::DoubleValue(v)),
            })
        }

        fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
            self.collect_str(&v)
        }

        fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
            Ok(AnyValue {
                value: Some(Value::StringValue(v.to_owned())),
            })
        }

        fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
            Ok(AnyValue {
                value: Some(Value::BytesValue(v.to_owned())),
            })
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Ok(AnyValue { value: None })
        }

        fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: serde::Serialize,
        {
            value.serialize(self)
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Ok(AnyValue { value: None })
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
            Ok(ValueSerializeSeq {
                value: ArrayValue { values: Vec::new() },
            })
        }

        fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Ok(ValueSerializeTuple {
                value: ArrayValue { values: Vec::new() },
            })
        }

        fn serialize_tuple_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Ok(ValueSerializeTupleStruct {
                value: ArrayValue { values: Vec::new() },
            })
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
                value: ArrayValue { values: Vec::new() },
            })
        }

        fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Ok(ValueSerializeMap {
                value: KeyValueList { values: Vec::new() },
            })
        }

        fn serialize_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            Ok(ValueSerializeStruct {
                value: KeyValueList { values: Vec::new() },
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
                value: KeyValueList { values: Vec::new() },
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
            Ok(self.value.values.push(value.serialize(ValueSerializer)?))
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(AnyValue {
                value: Some(Value::ArrayValue(self.value)),
            })
        }
    }

    impl SerializeTuple for ValueSerializeTuple {
        type Ok = AnyValue;

        type Error = ValueError;

        fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            Ok(self.value.values.push(value.serialize(ValueSerializer)?))
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(AnyValue {
                value: Some(Value::ArrayValue(self.value)),
            })
        }
    }

    impl SerializeTupleStruct for ValueSerializeTupleStruct {
        type Ok = AnyValue;

        type Error = ValueError;

        fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            Ok(self.value.values.push(value.serialize(ValueSerializer)?))
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(AnyValue {
                value: Some(Value::ArrayValue(self.value)),
            })
        }
    }

    impl SerializeTupleVariant for ValueSerializeTupleVariant {
        type Ok = AnyValue;

        type Error = ValueError;

        fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            self.value.values.push(value.serialize(ValueSerializer)?);

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(AnyValue {
                value: Some(Value::KvlistValue(KeyValueList {
                    values: vec![KeyValue {
                        key: self.variant.to_owned(),
                        value: Some(AnyValue {
                            value: Some(Value::ArrayValue(self.value)),
                        }),
                    }],
                })),
            })
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
                AnyValue {
                    value: Some(Value::StringValue(key)),
                } => key,
                key => format!("{:?}", key),
            };

            self.value.values.push(KeyValue { key, value: None });

            Ok(())
        }

        fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            self.value
                .values
                .last_mut()
                .ok_or_else(|| Error::custom("missing key"))?
                .value = Some(value.serialize(ValueSerializer)?);

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(AnyValue {
                value: Some(Value::KvlistValue(self.value)),
            })
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
            let key = key.to_owned();
            let value = Some(value.serialize(ValueSerializer)?);

            self.value.values.push(KeyValue { key, value });

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(AnyValue {
                value: Some(Value::KvlistValue(self.value)),
            })
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
            self.value.values.push(KeyValue {
                key: key.to_owned(),
                value: Some(value.serialize(ValueSerializer)?),
            });

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(AnyValue {
                value: Some(Value::KvlistValue(KeyValueList {
                    values: vec![KeyValue {
                        key: self.variant.to_owned(),
                        value: Some(AnyValue {
                            value: Some(Value::KvlistValue(self.value)),
                        }),
                    }],
                })),
            })
        }
    }
}
