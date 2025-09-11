//! This is a parser for JSON, using chumsky.

use core::fmt;
use std::borrow::Cow;
use std::collections::HashMap;

use std::collections::hash_map;

use chumsky::prelude::*;
use serde::de::{
    self, Deserialize, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor,
};

use crate::bms::prelude::{SimpleSource, ToAriadne};

use super::*;

/// JSON enum
#[derive(Clone, Debug, PartialEq)]
pub enum Json<'a> {
    /// Invalid JSON value (used for error recovery)
    Invalid,
    /// JSON null value
    Null,
    /// JSON boolean value
    Bool(bool),
    /// JSON string value
    Str(&'a str),
    /// JSON integer value
    Int(i64),
    /// JSON floating point value
    Float(f64),
    /// JSON array value
    Array(Vec<Json<'a>>),
    /// JSON object value
    Object(HashMap<&'a str, Json<'a>>),
}

fn parser<'a>() -> impl Parser<'a, &'a str, Json<'a>, extra::Err<Rich<'a, char>>> {
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
    ) -> ariadne::Report<'a, (&'a str, std::ops::Range<usize>)> {
        use ariadne::{Color, Label, Report, ReportKind};
        Report::build(ReportKind::Error, (src.name(), self.span().into_range()))
            .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
            .with_message(self.to_string())
            .with_label(
                Label::new((src.name(), self.span().into_range()))
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
/// assert_eq!(result, Some(Json::Str("hello world")));
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
///     assert_eq!(obj.get("key"), Some(&Json::Str("value")));
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
pub fn parse_json<'a>(src: &'a str) -> (Option<Json<'a>>, Vec<Rich<'a, char>>) {
    let (json, errs) = parser().parse(src.trim()).into_output_errors();
    (json, errs)
}

/// Entry point to deserialize any `T: Deserialize` from a `Json` value.
pub fn from_json<'a, T: for<'de> Deserialize<'de>>(
    json: &'a Json<'a>,
) -> Result<T, BmsonWarning<'static>> {
    let de = JsonDeserializer {
        json,
        path: Vec::new(),
    };
    T::deserialize(de)
}

struct JsonDeserializer<'a> {
    json: &'a Json<'a>,
    path: Vec<String>,
}

impl<'a> JsonDeserializer<'a> {
    fn fail(&self) -> BmsonWarning<'static> {
        let p = if self.path.is_empty() {
            Cow::Borrowed("root")
        } else {
            Cow::Owned(self.path.join("."))
        };
        BmsonWarning::DeserializeFailed(p)
    }
}

impl<'de, 'a> serde::Deserializer<'de> for JsonDeserializer<'a> {
    type Error = BmsonWarning<'static>;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Invalid => Err(self.fail()),
            Json::Null => visitor.visit_unit(),
            Json::Bool(b) => visitor.visit_bool(*b),
            Json::Str(s) => visitor.visit_string(s.to_string()),
            Json::Int(i) => visitor.visit_i64(*i),
            Json::Float(f) => visitor.visit_f64(*f),
            Json::Array(arr) => visitor.visit_seq(SeqAccessImpl {
                iter: arr.iter(),
                len: arr.len(),
                path: self.path.clone(),
                index: 0,
            }),
            Json::Object(obj) => {
                visitor.visit_map(MapAccessImpl::new(obj.iter(), self.path.clone()))
            }
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
            _ => Err(self.fail()),
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
            _ => Err(self.fail()),
        }
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Int(i) => visitor.visit_f32(*i as f32),
            Json::Float(f) => visitor.visit_f32(*f as f32),
            _ => Err(self.fail()),
        }
    }
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Int(i) => visitor.visit_f64(*i as f64),
            Json::Float(f) => visitor.visit_f64(*f),
            _ => Err(self.fail()),
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Str(s) => {
                let mut iter = s.chars();
                match (iter.next(), iter.next()) {
                    (Some(c), None) => visitor.visit_char(c),
                    _ => Err(self.fail()),
                }
            }
            _ => Err(self.fail()),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Str(s) => visitor.visit_str(s),
            _ => Err(self.fail()),
        }
    }
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.json {
            Json::Str(s) => visitor.visit_string(s.to_string()),
            _ => Err(self.fail()),
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
                    None => Err(self.fail()),
                }
            }
            _ => Err(self.fail()),
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
            _ => Err(self.fail()),
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
                path: self.path.clone(),
                index: 0,
            }),
            _ => Err(self.fail()),
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
            Json::Object(obj) => {
                visitor.visit_map(MapAccessImpl::new(obj.iter(), self.path.clone()))
            }
            _ => Err(self.fail()),
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
                    (Some((k, v)), None) => {
                        visitor.visit_enum(EnumAccessImpl::Tagged(k, v, self.path.clone()))
                    }
                    _ => Err(self.fail()),
                }
            }
            _ => Err(self.fail()),
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
    iter: std::slice::Iter<'a, Json<'a>>,
    len: usize,
    path: Vec<String>,
    index: usize,
}

impl<'de, 'a> SeqAccess<'de> for SeqAccessImpl<'a> {
    type Error = BmsonWarning<'static>;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        match self.iter.next() {
            Some(next) => {
                let idx = self.index;
                self.index += 1;
                let mut p = self.path.clone();
                p.push(format!("[{}]", idx));
                seed.deserialize(JsonDeserializer {
                    json: next,
                    path: p,
                })
                .map(Some)
            }
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

struct MapAccessImpl<'a> {
    iter: hash_map::Iter<'a, &'a str, Json<'a>>,
    next_value: Option<&'a Json<'a>>,
    path: Vec<String>,
    current_key: Option<&'a str>,
}

impl<'a> MapAccessImpl<'a> {
    fn new(iter: hash_map::Iter<'a, &'a str, Json<'a>>, path: Vec<String>) -> Self {
        Self {
            iter,
            next_value: None,
            path,
            current_key: None,
        }
    }
}

impl<'de, 'a> MapAccess<'de> for MapAccessImpl<'a> {
    type Error = BmsonWarning<'static>;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        match self.iter.next() {
            Some((k, v)) => {
                self.current_key = Some(*k);
                self.next_value = Some(v);
                seed.deserialize((*k).into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<Vv: DeserializeSeed<'de>>(
        &mut self,
        seed: Vv,
    ) -> Result<Vv::Value, Self::Error> {
        match self.next_value.take() {
            Some(v) => {
                let mut p = self.path.clone();
                if let Some(k) = self.current_key.take() {
                    p.push(k.to_string());
                }
                seed.deserialize(JsonDeserializer { json: v, path: p })
            }
            None => Err(BmsonWarning::DeserializeFailed(if self.path.is_empty() {
                Cow::Borrowed("root")
            } else {
                Cow::Owned(self.path.join("."))
            })),
        }
    }
}

enum EnumAccessImpl<'a> {
    Unit(JsonDeserializer<'a>),
    Tagged(&'a str, &'a Json<'a>, Vec<String>),
}

impl<'de, 'a> de::EnumAccess<'de> for EnumAccessImpl<'a> {
    type Error = BmsonWarning<'static>;
    type Variant = VariantAccessImpl<'a>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        match self {
            EnumAccessImpl::Unit(de) => {
                // Variant is a simple string or integer
                let v = seed.deserialize(de)?;
                Ok((
                    v,
                    VariantAccessImpl {
                        value: None,
                        path: vec![],
                    },
                ))
            }
            EnumAccessImpl::Tagged(k, v, path) => {
                let variant = seed.deserialize((*k).into_deserializer())?;
                Ok((
                    variant,
                    VariantAccessImpl {
                        value: Some(v),
                        path,
                    },
                ))
            }
        }
    }
}

struct VariantAccessImpl<'a> {
    value: Option<&'a Json<'a>>,
    path: Vec<String>,
}

impl<'de, 'a> de::VariantAccess<'de> for VariantAccessImpl<'a> {
    type Error = BmsonWarning<'static>;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.value {
            None | Some(Json::Null) => Ok(()),
            _ => Err(BmsonWarning::DeserializeFailed(if self.path.is_empty() {
                Cow::Borrowed("root")
            } else {
                Cow::Owned(self.path.join("."))
            })),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        match self.value {
            Some(v) => {
                let p = self.path.clone();
                seed.deserialize(JsonDeserializer { json: v, path: p })
            }
            None => Err(BmsonWarning::DeserializeFailed(if self.path.is_empty() {
                Cow::Borrowed("root")
            } else {
                Cow::Owned(self.path.join("."))
            })),
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
                path: self.path.clone(),
                index: 0,
            }),
            _ => Err(BmsonWarning::DeserializeFailed(if self.path.is_empty() {
                Cow::Borrowed("root")
            } else {
                Cow::Owned(self.path.join("."))
            })),
        }
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.value {
            Some(Json::Object(obj)) => {
                visitor.visit_map(MapAccessImpl::new(obj.iter(), self.path.clone()))
            }
            _ => Err(BmsonWarning::DeserializeFailed(if self.path.is_empty() {
                Cow::Borrowed("root")
            } else {
                Cow::Owned(self.path.join("."))
            })),
        }
    }
}

fn default_bmson() -> Bmson<'static> {
    // 最终兜底：返回空的占位值（尽力而为，避免 unwrap）
    Bmson {
        version: Cow::Borrowed("1.0.0"),
        info: BmsonInfo {
            title: Cow::Borrowed(""),
            subtitle: Cow::Borrowed(""),
            artist: Cow::Borrowed(""),
            subartists: Vec::new(),
            genre: Cow::Borrowed(""),
            mode_hint: default_mode_hint(),
            chart_name: Cow::Borrowed(""),
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

fn minimal_bmson_json() -> Json<'static> {
    let mut min_root: HashMap<&'static str, Json<'static>> = HashMap::new();
    let mut min_info: HashMap<&'static str, Json<'static>> = HashMap::new();
    min_root.insert("version", Json::Str("1.0.0"));
    min_root.insert("sound_channels", Json::Array(Vec::new()));
    min_info.insert("title", Json::Str(""));
    min_info.insert("artist", Json::Str(""));
    min_info.insert("genre", Json::Str(""));
    min_info.insert("level", Json::Int(0));
    min_info.insert("init_bpm", Json::Float(120.0));
    min_root.insert("info", Json::Object(min_info));
    Json::Object(min_root)
}

/// bmson 解析时的告警/错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BmsonWarning<'a> {
    /// JSON 语法错误（直接返回 Rich）。
    JsonSyntaxError(Rich<'a, char>),
    /// 根节点不是对象。
    NonObjectRoot,
    /// 缺失字段（使用默认值填充）。
    MissingField(Cow<'a, str>),
    /// 反序列化失败（标注具体字段路径）。
    DeserializeFailed(Cow<'a, str>),
}

impl<'a> fmt::Display for BmsonWarning<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BmsonWarning::JsonSyntaxError(rich) => write!(f, "json syntax error: {}", rich),
            BmsonWarning::NonObjectRoot => write!(f, "root is not an object"),
            BmsonWarning::MissingField(name) => write!(f, "missing field: {}", name),
            BmsonWarning::DeserializeFailed(path) => write!(f, "deserialize failed at: {}", path),
        }
    }
}

impl<'a> std::error::Error for BmsonWarning<'a> {}

impl<'a> de::Error for BmsonWarning<'a> {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        BmsonWarning::DeserializeFailed(Cow::Owned(msg.to_string()))
    }
}

/// `parse_bmson` 的输出。
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct BmsonOutput<'a> {
    /// 解析后的 bmson 对象。
    pub bmson: Bmson<'a>,
    /// 解析过程中的警告。
    pub warnings: Vec<BmsonWarning<'a>>,
}

/// 解析 bmson 源字符串。不使用 `serde_json`，完全基于 chumsky + serde 反序列化。
#[must_use]
pub(crate) fn parse_bmson_inner<'a>(src: &'a str) -> BmsonOutput<'a> {
    let (maybe_json, errs) = parse_json(src);
    let mut warnings: Vec<BmsonWarning<'a>> = Vec::new();
    if !errs.is_empty() {
        warnings.extend(errs.into_iter().map(BmsonWarning::JsonSyntaxError));
    }

    // 准备对象根
    let mut root_map: HashMap<&'a str, Json<'a>> = match maybe_json {
        Some(Json::Object(ref m)) => m.clone(),
        _ => {
            warnings.push(BmsonWarning::NonObjectRoot);
            HashMap::new()
        }
    };

    // 顶层缺失字段默认
    if !root_map.contains_key("version") {
        warnings.push(BmsonWarning::MissingField(Cow::Borrowed("version")));
        root_map.insert("version", Json::Str("1.0.0"));
    }
    if !root_map.contains_key("sound_channels") {
        warnings.push(BmsonWarning::MissingField(Cow::Borrowed("sound_channels")));
        root_map.insert("sound_channels", Json::Array(Vec::new()));
    }

    // info 对象缺省与字段默认
    let info_json = root_map.remove("info");
    let mut info_map: HashMap<&'a str, Json<'a>> = match info_json {
        Some(Json::Object(m)) => m,
        _ => {
            warnings.push(BmsonWarning::MissingField(Cow::Borrowed("info")));
            HashMap::new()
        }
    };
    if !info_map.contains_key("title") {
        warnings.push(BmsonWarning::MissingField(Cow::Borrowed("info.title")));
        info_map.insert("title", Json::Str(""));
    }
    if !info_map.contains_key("artist") {
        warnings.push(BmsonWarning::MissingField(Cow::Borrowed("info.artist")));
        info_map.insert("artist", Json::Str(""));
    }
    if !info_map.contains_key("genre") {
        warnings.push(BmsonWarning::MissingField(Cow::Borrowed("info.genre")));
        info_map.insert("genre", Json::Str(""));
    }
    if !info_map.contains_key("level") {
        warnings.push(BmsonWarning::MissingField(Cow::Borrowed("info.level")));
        info_map.insert("level", Json::Int(0));
    }
    if !info_map.contains_key("init_bpm") {
        warnings.push(BmsonWarning::MissingField(Cow::Borrowed("info.init_bpm")));
        info_map.insert("init_bpm", Json::Float(120.0));
    }
    root_map.insert("info", Json::Object(info_map));

    // 反序列化
    let json_root = Json::Object(root_map);
    match from_json::<Bmson<'a>>(&json_root) {
        Ok(bmson) => BmsonOutput { bmson, warnings },
        Err(e) => {
            warnings.push(e);
            // 构造保证可反序列化的最小对象
            let min_json = minimal_bmson_json();
            let bmson = match from_json::<Bmson<'a>>(&min_json) {
                Ok(b) => b,
                Err(_) => {
                    // 理论上不会失败；退化到直接复用最小 JSON 再尝试一次
                    match from_json::<Bmson<'a>>(&min_json) {
                        Ok(b) => b,
                        Err(_) => default_bmson(),
                    }
                }
            };
            BmsonOutput { bmson, warnings }
        }
    }
}
