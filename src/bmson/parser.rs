//! This is a parser for JSON, using chumsky.

use chumsky::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]

/// JSON enum
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

/// Create a report for a rich error.
pub fn rich_err<'a>(e: &Rich<'a, char>) -> ariadne::Report<'a, ((), std::ops::Range<usize>)> {
    use ariadne::{Color, Label, Report, ReportKind};
    Report::build(ReportKind::Error, ((), e.span().into_range()))
        .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
        .with_message(e.to_string())
        .with_label(
            Label::new(((), e.span().into_range()))
                .with_message(e.reason().to_string())
                .with_color(Color::Red),
        )
        .finish()
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
