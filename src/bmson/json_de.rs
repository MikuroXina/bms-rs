//! Serde Deserializer over the chumsky-based JSON AST.
//!
//! This allows deserializing existing `#[derive(Deserialize)]` structs from
//! `crate::bmson::parser::Json` without using `serde_json`.

use core::fmt;
use std::collections::hash_map;

use serde::de::{
    self, Deserialize, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor,
};

use super::parser::Json;

/// Error type for JSON deserialization over `Json` AST.
#[derive(Debug)]
pub struct DeError {
    message: String,
}

impl DeError {
    pub fn custom<T: fmt::Display>(msg: T) -> Self {
        Self {
            message: msg.to_string(),
        }
    }
}

impl de::Error for DeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self {
            message: msg.to_string(),
        }
    }
}

impl fmt::Display for DeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DeError {}

/// Entry point to deserialize any `T: Deserialize` from a `Json` value.
pub fn from_json<T: for<'de> Deserialize<'de>>(json: &Json) -> Result<T, DeError> {
    let de = JsonDeserializer { json };
    T::deserialize(de)
}

struct JsonDeserializer<'a> {
    json: &'a Json,
}

impl<'de, 'a> serde::Deserializer<'de> for JsonDeserializer<'a> {
    type Error = DeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Invalid => Err(<DeError as de::Error>::custom("invalid JSON value")),
            Json::Null => visitor.visit_unit(),
            Json::Bool(b) => visitor.visit_bool(*b),
            Json::Str(s) => visitor.visit_string(s.clone()),
            Json::Int(i) => visitor.visit_i64(*i),
            Json::Float(f) => visitor.visit_f64(*f),
            Json::Array(arr) => visitor.visit_seq(SeqAccessImpl {
                iter: arr.iter(),
                len: arr.len(),
            }),
            Json::Object(obj) => visitor.visit_map(MapAccessImpl::new(obj.iter())),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Bool(b) => visitor.visit_bool(*b),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Int(i) => visitor.visit_i64(*i),
            Json::Float(f) => visitor.visit_i64(*f as i64),
            _ => Err(<DeError as de::Error>::custom("expected integer")),
        }
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u64(visitor)
    }
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u64(visitor)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u64(visitor)
    }
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Int(i) if *i >= 0 => visitor.visit_u64(*i as u64),
            Json::Float(f) if *f >= 0.0 => visitor.visit_u64(*f as u64),
            _ => Err(<DeError as de::Error>::custom("expected unsigned integer")),
        }
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Int(i) => visitor.visit_f32(*i as f32),
            Json::Float(f) => visitor.visit_f32(*f as f32),
            _ => Err(<DeError as de::Error>::custom("expected float")),
        }
    }
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Int(i) => visitor.visit_f64(*i as f64),
            Json::Float(f) => visitor.visit_f64(*f),
            _ => Err(<DeError as de::Error>::custom("expected float")),
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Str(s) => {
                let mut iter = s.chars();
                match (iter.next(), iter.next()) {
                    (Some(c), None) => visitor.visit_char(c),
                    _ => Err(<DeError as de::Error>::custom(
                        "expected single-character string",
                    )),
                }
            }
            _ => Err(<DeError as de::Error>::custom("expected char")),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Str(s) => visitor.visit_str(s),
            _ => Err(<DeError as de::Error>::custom("expected string")),
        }
    }
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Str(s) => visitor.visit_string(s.clone()),
            _ => Err(<DeError as de::Error>::custom("expected string")),
        }
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Str(s) => visitor.visit_bytes(s.as_bytes()),
            Json::Array(arr) => {
                let bytes: Option<Vec<u8>> = arr
                    .iter()
                    .map(|j| match j {
                        Json::Int(i) if *i >= 0 && *i <= 255 => Some(*i as u8),
                        _ => None,
                    })
                    .collect();
                match bytes {
                    Some(v) => visitor.visit_byte_buf(v),
                    None => Err(<DeError as de::Error>::custom("expected byte array")),
                }
            }
            _ => Err(<DeError as de::Error>::custom("expected bytes")),
        }
    }
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Null => visitor.visit_unit(),
            _ => Err(<DeError as de::Error>::custom("expected null")),
        }
    }
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_unit(visitor)
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Array(arr) => visitor.visit_seq(SeqAccessImpl {
                iter: arr.iter(),
                len: arr.len(),
            }),
            _ => Err(<DeError as de::Error>::custom("expected array")),
        }
    }
    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Object(obj) => visitor.visit_map(MapAccessImpl::new(obj.iter())),
            _ => Err(<DeError as de::Error>::custom("expected object")),
        }
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Str(_) | Json::Int(_) => visitor.visit_enum(EnumAccessImpl::Unit(self)),
            Json::Object(obj) => {
                // Externally tagged: { "Variant": value }
                let mut iter = obj.iter();
                match (iter.next(), iter.next()) {
                    (Some((k, v)), None) => visitor.visit_enum(EnumAccessImpl::Tagged(k, v)),
                    _ => Err(<DeError as de::Error>::custom(
                        "expected single-key object for enum",
                    )),
                }
            }
            _ => Err(<DeError as de::Error>::custom(
                "invalid enum representation",
            )),
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }
}

struct SeqAccessImpl<'a> {
    iter: std::slice::Iter<'a, Json>,
    len: usize,
}

impl<'de, 'a> SeqAccess<'de> for SeqAccessImpl<'a> {
    type Error = DeError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        match self.iter.next() {
            Some(next) => seed.deserialize(JsonDeserializer { json: next }).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

struct MapAccessImpl<'a> {
    iter: hash_map::Iter<'a, String, Json>,
    next_value: Option<&'a Json>,
}

impl<'a> MapAccessImpl<'a> {
    fn new(iter: hash_map::Iter<'a, String, Json>) -> Self {
        Self {
            iter,
            next_value: None,
        }
    }
}

impl<'de, 'a> MapAccess<'de> for MapAccessImpl<'a> {
    type Error = DeError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        match self.iter.next() {
            Some((k, v)) => {
                self.next_value = Some(v);
                seed.deserialize(k.as_str().into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<Vv: DeserializeSeed<'de>>(
        &mut self,
        seed: Vv,
    ) -> Result<Vv::Value, Self::Error> {
        match self.next_value.take() {
            Some(v) => seed.deserialize(JsonDeserializer { json: v }),
            None => Err(<DeError as de::Error>::custom("value is missing for key")),
        }
    }
}

enum EnumAccessImpl<'a> {
    Unit(JsonDeserializer<'a>),
    Tagged(&'a String, &'a Json),
}

impl<'de, 'a> de::EnumAccess<'de> for EnumAccessImpl<'a> {
    type Error = DeError;
    type Variant = VariantAccessImpl<'a>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        match self {
            EnumAccessImpl::Unit(de) => {
                // Variant is a simple string or integer
                let v = seed.deserialize(de)?;
                Ok((v, VariantAccessImpl { value: None }))
            }
            EnumAccessImpl::Tagged(k, v) => {
                let variant = seed.deserialize(k.as_str().into_deserializer())?;
                Ok((variant, VariantAccessImpl { value: Some(v) }))
            }
        }
    }
}

struct VariantAccessImpl<'a> {
    value: Option<&'a Json>,
}

impl<'de, 'a> de::VariantAccess<'de> for VariantAccessImpl<'a> {
    type Error = DeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.value {
            None | Some(Json::Null) => Ok(()),
            _ => Err(<DeError as de::Error>::custom("expected unit variant")),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        match self.value {
            Some(v) => seed.deserialize(JsonDeserializer { json: v }),
            None => Err(<DeError as de::Error>::custom(
                "expected newtype variant value",
            )),
        }
    }

    fn tuple_variant<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.value {
            Some(Json::Array(arr)) => visitor.visit_seq(SeqAccessImpl {
                iter: arr.iter(),
                len: arr.len(),
            }),
            _ => Err(<DeError as de::Error>::custom("expected tuple variant")),
        }
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.value {
            Some(Json::Object(obj)) => visitor.visit_map(MapAccessImpl::new(obj.iter())),
            _ => Err(<DeError as de::Error>::custom("expected struct variant")),
        }
    }
}
