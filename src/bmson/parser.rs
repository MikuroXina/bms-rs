//! This is a parser for JSON, using chumsky.

use chumsky::prelude::*;
use std::collections::HashMap;

use core::fmt;
use std::collections::hash_map;

use serde::de::{
    self, Deserialize, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor,
};

use crate::bms::prelude::{SimpleSource, ToAriadne};

use super::*;

/// JSON enum
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    /// Invalid JSON value (used for error recovery)
    Invalid,
    /// JSON null value
    Null,
    /// JSON boolean value
    Bool(bool),
    /// JSON string value
    Str(String),
    /// JSON integer value
    Int(i64),
    /// JSON floating point value
    Float(f64),
    /// JSON array value
    Array(Vec<Json>),
    /// JSON object value
    Object(HashMap<String, Json>),
}

fn parser<'a>() -> impl Parser<'a, &'a str, Json, extra::Err<Rich<'a, char>>> {
    recursive(|value| {
        let digits = text::digits(10).to_slice();

        let frac = just('.').then(digits);

        let exp = just('e')
            .or(just('E'))
            .then(one_of("+-").or_not())
            .then(digits);

        let number = just('-')
            .or_not()
            .then(text::int(10))
            .then(frac.or_not())
            .then(exp.or_not())
            .to_slice()
            .map(|s: &str| {
                // Check if it's a floating point number (contains '.' or 'e'/'E')
                if s.contains('.') || s.to_lowercase().contains('e') {
                    Json::Float(s.parse().unwrap())
                } else {
                    Json::Int(s.parse().unwrap())
                }
            })
            .boxed();

        let escape = just('\\')
            .then(choice((
                just('\\'),
                just('/'),
                just('"'),
                just('b').to('\x08'),
                just('f').to('\x0C'),
                just('n').to('\n'),
                just('r').to('\r'),
                just('t').to('\t'),
                just('u').ignore_then(text::digits(16).exactly(4).to_slice().validate(
                    |digits, e, emitter| {
                        char::from_u32(u32::from_str_radix(digits, 16).unwrap()).unwrap_or_else(
                            || {
                                emitter.emit(Rich::custom(e.span(), "invalid unicode character"));
                                '\u{FFFD}' // unicode replacement character
                            },
                        )
                    },
                )),
            )))
            .ignored()
            .boxed();

        let string = none_of("\\\"")
            .ignored()
            .or(escape)
            .repeated()
            .to_slice()
            .map(ToString::to_string)
            .delimited_by(just('"'), just('"'))
            .boxed();

        let array = value
            .clone()
            .separated_by(just(',').padded().recover_with(skip_then_retry_until(
                any().ignored(),
                one_of(",]").ignored(),
            )))
            .allow_trailing()
            .collect()
            .padded()
            .delimited_by(
                just('['),
                just(']')
                    .ignored()
                    .recover_with(via_parser(end()))
                    .recover_with(skip_then_retry_until(any().ignored(), end())),
            )
            .boxed();

        let member = string.clone().then_ignore(just(':').padded()).then(value);
        let object = member
            .clone()
            .separated_by(just(',').padded().recover_with(skip_then_retry_until(
                any().ignored(),
                one_of(",}").ignored(),
            )))
            .collect()
            .padded()
            .delimited_by(
                just('{'),
                just('}')
                    .ignored()
                    .recover_with(via_parser(end()))
                    .recover_with(skip_then_retry_until(any().ignored(), end())),
            )
            .boxed();

        choice((
            just("null").to(Json::Null),
            just("true").to(Json::Bool(true)),
            just("false").to(Json::Bool(false)),
            number,
            string.map(Json::Str),
            array.map(Json::Array),
            object.map(Json::Object),
        ))
        .recover_with(via_parser(nested_delimiters(
            '{',
            '}',
            [('[', ']')],
            |_| Json::Invalid,
        )))
        .recover_with(via_parser(nested_delimiters(
            '[',
            ']',
            [('{', '}')],
            |_| Json::Invalid,
        )))
        .recover_with(skip_then_retry_until(
            any().ignored(),
            one_of(",]}").ignored(),
        ))
        .padded()
    })
}

impl ToAriadne for Rich<'_, char> {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> ariadne::Report<'a, (String, std::ops::Range<usize>)> {
        use ariadne::{Color, Label, Report, ReportKind};
        Report::build(
            ReportKind::Error,
            (src.name().to_string(), self.span().into_range()),
        )
        .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
        .with_message(self.to_string())
        .with_label(
            Label::new((src.name().to_string(), self.span().into_range()))
                .with_message(self.reason().to_string())
                .with_color(Color::Red),
        )
        .finish()
    }
}

/// Parse a JSON string and return the result and errors.
///
/// # Examples
///
/// Basic data types:
/// ```
/// # use bms_rs::bmson::parser::{parse_json, Json};
/// let (result, errors) = parse_json("null");
/// assert!(errors.is_empty());
/// assert_eq!(result, Some(Json::Null));
/// ```
///
/// ```
/// # use bms_rs::bmson::parser::{parse_json, Json};
/// let (result, errors) = parse_json("true");
/// assert!(errors.is_empty());
/// assert_eq!(result, Some(Json::Bool(true)));
/// ```
///
/// ```
/// # use bms_rs::bmson::parser::{parse_json, Json};
/// let (result, errors) = parse_json("\"hello world\"");
/// assert!(errors.is_empty());
/// assert_eq!(result, Some(Json::Str("hello world".to_string())));
/// ```
///
/// ```
/// # use bms_rs::bmson::parser::{parse_json, Json};
/// let (result, errors) = parse_json("42");
/// assert!(errors.is_empty());
/// assert_eq!(result, Some(Json::Int(42)));
/// ```
///
/// ```
/// # use bms_rs::bmson::parser::{parse_json, Json};
/// let (result, errors) = parse_json("3.14");
/// assert!(errors.is_empty());
/// if let Some(Json::Float(f)) = result {
///     assert!((f - 3.14).abs() < f64::EPSILON);
/// }
/// ```
///
/// Arrays:
/// ```
/// # use bms_rs::bmson::parser::{parse_json, Json};
/// let (result, errors) = parse_json("[1, 2, 3]");
/// assert!(errors.is_empty());
/// if let Some(Json::Array(arr)) = result {
///     assert_eq!(arr.len(), 3);
/// }
/// ```
///
/// Objects:
/// ```
/// # use bms_rs::bmson::parser::{parse_json, Json};
/// let (result, errors) = parse_json("{\"key\": \"value\"}");
/// assert!(errors.is_empty());
/// if let Some(Json::Object(obj)) = result {
///     assert_eq!(obj.get("key"), Some(&Json::Str("value".to_string())));
/// }
/// ```
///
/// Complex nested structure:
/// ```
/// # use bms_rs::bmson::parser::{parse_json, Json};
/// let (result, errors) = parse_json("{\"items\": [1, 2, {\"nested\": true}]}");
/// assert!(errors.is_empty());
/// assert!(result.is_some());
/// ```
///
/// Invalid JSON (returns errors):
/// ```
/// # use bms_rs::bmson::parser::parse_json;
/// let (result, errors) = parse_json("{invalid json");
/// assert!(result.is_none());
/// assert!(!errors.is_empty());
/// ```
pub fn parse_json(src: &str) -> (Option<Json>, Vec<Rich<'_, char>>) {
    let (json, errs) = parser().parse(src.trim()).into_output_errors();
    (json, errs)
}

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

fn default_bmson() -> Bmson {
    // 最终兜底：返回空的占位值（尽力而为，避免 unwrap）
    Bmson {
        version: "1.0.0".to_string(),
        info: BmsonInfo {
            title: String::new(),
            subtitle: String::new(),
            artist: String::new(),
            subartists: Vec::new(),
            genre: String::new(),
            mode_hint: default_mode_hint(),
            chart_name: String::new(),
            level: 0,
            init_bpm: FinF64::try_from(120.0).unwrap_or_else(|_| FinF64::try_from(0.0).unwrap()),
            judge_rank: default_percentage(),
            total: default_percentage(),
            back_image: None,
            eyecatch_image: None,
            title_image: None,
            banner_image: None,
            preview_music: None,
            resolution: default_resolution(),
            ln_type: LnMode::default(),
        },
        lines: None,
        bpm_events: Vec::new(),
        stop_events: Vec::new(),
        sound_channels: Vec::new(),
        bga: Bga::default(),
        scroll_events: Vec::new(),
        mine_channels: Vec::new(),
        key_channels: Vec::new(),
    }
}

/// bmson 解析时的告警/错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BmsonWarning {
    /// JSON 语法错误的数量。
    JsonSyntaxErrorCount(usize),
    /// 根节点不是对象。
    NonObjectRoot,
    /// 缺失字段（使用默认值填充）。
    MissingField(&'static str),
    /// 反序列化失败（类型不匹配等）。
    DeserializeFailed,
}

/// `parse_bmson` 的输出。
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct BmsonOutput {
    /// 解析后的 bmson 对象。
    pub bmson: Bmson,
    /// 解析过程中的警告。
    pub warnings: Vec<BmsonWarning>,
}

/// 解析 bmson 源字符串。不使用 `serde_json`，完全基于 chumsky + serde 反序列化。
#[must_use]
pub(crate) fn parse_bmson_inner(src: &str) -> BmsonOutput {
    let (maybe_json, errs) = parse_json(src);
    let mut warnings: Vec<BmsonWarning> = Vec::new();
    if !errs.is_empty() {
        warnings.push(BmsonWarning::JsonSyntaxErrorCount(errs.len()));
    }

    // 准备对象根
    let mut root_map: HashMap<String, Json> = match maybe_json {
        Some(Json::Object(ref m)) => m.clone(),
        _ => {
            warnings.push(BmsonWarning::NonObjectRoot);
            HashMap::new()
        }
    };

    // 顶层缺失字段默认
    if !root_map.contains_key("version") {
        warnings.push(BmsonWarning::MissingField("version"));
        root_map.insert("version".into(), Json::Str("1.0.0".into()));
    }
    if !root_map.contains_key("sound_channels") {
        warnings.push(BmsonWarning::MissingField("sound_channels"));
        root_map.insert("sound_channels".into(), Json::Array(Vec::new()));
    }

    // info 对象缺省与字段默认
    let info_json = root_map.remove("info");
    let mut info_map: HashMap<String, Json> = match info_json {
        Some(Json::Object(m)) => m,
        _ => {
            warnings.push(BmsonWarning::MissingField("info"));
            HashMap::new()
        }
    };
    if !info_map.contains_key("title") {
        warnings.push(BmsonWarning::MissingField("info.title"));
        info_map.insert("title".into(), Json::Str(String::new()));
    }
    if !info_map.contains_key("artist") {
        warnings.push(BmsonWarning::MissingField("info.artist"));
        info_map.insert("artist".into(), Json::Str(String::new()));
    }
    if !info_map.contains_key("genre") {
        warnings.push(BmsonWarning::MissingField("info.genre"));
        info_map.insert("genre".into(), Json::Str(String::new()));
    }
    if !info_map.contains_key("level") {
        warnings.push(BmsonWarning::MissingField("info.level"));
        info_map.insert("level".into(), Json::Int(0));
    }
    if !info_map.contains_key("init_bpm") {
        warnings.push(BmsonWarning::MissingField("info.init_bpm"));
        info_map.insert("init_bpm".into(), Json::Float(120.0));
    }
    root_map.insert("info".into(), Json::Object(info_map));

    // 反序列化
    let json_root = Json::Object(root_map);
    match from_json::<Bmson>(&json_root) {
        Ok(bmson) => BmsonOutput { bmson, warnings },
        Err(_) => {
            warnings.push(BmsonWarning::DeserializeFailed);
            // 构造保证可反序列化的最小对象
            let mut min_root = HashMap::new();
            let mut min_info = HashMap::new();
            min_root.insert("version".into(), Json::Str("1.0.0".into()));
            min_root.insert("sound_channels".into(), Json::Array(Vec::new()));
            min_info.insert("title".into(), Json::Str(String::new()));
            min_info.insert("artist".into(), Json::Str(String::new()));
            min_info.insert("genre".into(), Json::Str(String::new()));
            min_info.insert("level".into(), Json::Int(0));
            min_info.insert("init_bpm".into(), Json::Float(120.0));
            min_root.insert("info".into(), Json::Object(min_info));
            let min_json = Json::Object(min_root);
            let bmson = match from_json::<Bmson>(&min_json) {
                Ok(b) => b,
                Err(_) => {
                    // 理论上不会失败；退化到直接复用最小 JSON 再尝试一次
                    match from_json::<Bmson>(&min_json) {
                        Ok(b) => b,
                        Err(_) => default_bmson(),
                    }
                }
            };
            BmsonOutput { bmson, warnings }
        }
    }
}
