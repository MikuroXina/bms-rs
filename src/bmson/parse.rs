//! This is a parser for JSON.

use chumsky::{error::RichReason, prelude::*};
use serde_json::Value;

#[cfg(feature = "diagnostics")]
use ariadne::{Color, Report, ReportKind};

#[cfg(feature = "diagnostics")]
use crate::diagnostics::{SimpleSource, ToAriadne, build_report};

/// This is a parser for JSON.
///
/// Parsing from str, returning [`Value`]. Chumsky emits `Rich<char>` internally,
/// which we later classify into `Warning` (custom diagnostics) and `Recovered`
/// (grammar errors recovered by the parser).
#[must_use]
pub fn parser<'a>() -> impl Parser<'a, &'a str, Value, extra::Err<Rich<'a, char>>> {
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
                // Try to parse as integer first, then as float
                s.parse::<i64>()
                    .map(|i| Value::Number(serde_json::Number::from(i)))
                    .or_else(|_| {
                        s.parse::<f64>().map(|f| {
                            Value::Number(
                                serde_json::Number::from_f64(f)
                                    .unwrap_or_else(|| serde_json::Number::from(0)),
                            )
                        })
                    })
                    .unwrap_or_else(|_| Value::Number(serde_json::Number::from(0)))
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

        let member = string
            .clone()
            .then_ignore(just(':').padded())
            .then(value.clone());

        // Support objects with:
        // - normal commas
        // - missing commas between members (emit an error but continue)
        // - a trailing comma before the closing '}'
        let subsequent_member = choice((
            // Normal: comma then member
            just(',').padded().ignore_then(member.clone()).map(Some),
            // Missing comma: directly another member. Emit an error and continue.
            member
                .clone()
                .validate(|m, e, emitter| {
                    emitter.emit(Rich::custom(
                        e.span(),
                        "expected ',' between object members",
                    ));
                    m
                })
                .map(Some),
            // Trailing comma: consume it and yield no item
            just(',').padded().to::<Option<(String, Value)>>(None),
        ));

        let members = member
            .clone()
            .or_not()
            .then(subsequent_member.repeated().collect::<Vec<_>>())
            .map(|(first_opt, rest)| {
                let mut pairs: Vec<(String, Value)> = Vec::new();
                if let Some(first) = first_opt {
                    pairs.push(first);
                }
                for item in rest.into_iter().flatten() {
                    pairs.push(item);
                }
                pairs
            });

        let object = members
            .map(|pairs| {
                let mut map = serde_json::Map::new();
                for (key, value) in pairs {
                    map.insert(key, value);
                }
                Value::Object(map)
            })
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
            just("null").to(Value::Null),
            just("true").to(Value::Bool(true)),
            just("false").to(Value::Bool(false)),
            number,
            string.map(Value::String),
            array.map(Value::Array),
            object,
        ))
        .recover_with(via_parser(nested_delimiters(
            '{',
            '}',
            [('[', ']')],
            |_| Value::Null,
        )))
        .recover_with(via_parser(nested_delimiters(
            '[',
            ']',
            [('{', '}')],
            |_| Value::Null,
        )))
        .recover_with(skip_then_retry_until(
            any().ignored(),
            one_of(",]}").ignored(),
        ))
        .padded()
    })
}

/// Error recovered by the JSON parser. These originated from grammar mismatches
/// that were recovered via `recover_with` or similar mechanisms.
#[derive(Debug, Clone)]
pub struct Recovered<'a>(pub Rich<'a, char>);

/// Diagnostic warning intentionally emitted by the JSON parser using `Rich::custom`.
#[derive(Debug, Clone)]
pub struct Warning<'a>(pub Rich<'a, char>);

/// Unrecoverable JSON parsing error (no output value was produced).
#[derive(Debug, Clone)]
pub struct Error<'a>(pub Rich<'a, char>);

#[cfg(feature = "diagnostics")]
impl<'a> ToAriadne for Recovered<'a> {
    fn to_report<'b>(
        &self,
        src: &SimpleSource<'b>,
    ) -> Report<'b, (String, std::ops::Range<usize>)> {
        let span = self.0.span();
        let message = self.0.to_string();
        build_report(
            src,
            ReportKind::Advice,
            span.start..span.end,
            "JSON recovered parsing issue",
            &message,
            Color::Blue,
        )
    }
}

#[cfg(feature = "diagnostics")]
impl<'a> ToAriadne for Warning<'a> {
    fn to_report<'b>(
        &self,
        src: &SimpleSource<'b>,
    ) -> Report<'b, (String, std::ops::Range<usize>)> {
        let span = self.0.span();
        let message = self.0.to_string();
        build_report(
            src,
            ReportKind::Warning,
            span.start..span.end,
            "JSON parsing warning",
            &message,
            Color::Yellow,
        )
    }
}

#[cfg(feature = "diagnostics")]
impl<'a> ToAriadne for Error<'a> {
    fn to_report<'b>(
        &self,
        src: &SimpleSource<'b>,
    ) -> Report<'b, (String, std::ops::Range<usize>)> {
        let span = self.0.span();
        let message = self.0.to_string();
        build_report(
            src,
            ReportKind::Error,
            span.start..span.end,
            "JSON parsing error",
            &message,
            Color::Red,
        )
    }
}

/// Split chumsky `Rich<char>` errors into `Warning`, `Recovered`, and `Error` buckets.
#[must_use]
pub fn split_chumsky_errors<'a>(
    errors: impl IntoIterator<Item = Rich<'a, char>>,
    had_output: bool,
) -> (Vec<Warning<'a>>, Vec<Recovered<'a>>, Vec<Error<'a>>) {
    let mut warnings = Vec::new();
    let mut recovered = Vec::new();
    let mut fatal = Vec::new();
    for err in errors {
        match err.reason() {
            // Custom reasons are produced via `Rich::custom(...)` in this module,
            // which we treat as non-fatal parser diagnostics.
            RichReason::Custom(_) => warnings.push(Warning(err)),
            // All other errors: recovered if we produced an output value, otherwise fatal.
            _ if had_output => recovered.push(Recovered(err)),
            _ => fatal.push(Error(err)),
        }
    }
    (warnings, recovered, fatal)
}
