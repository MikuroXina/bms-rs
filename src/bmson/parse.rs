//! This is a parser for JSON.

use ariadne::{Color, Label, Report, ReportKind};
use chumsky::prelude::*;
use serde_json::Value;

use crate::bms::diagnostics::{SimpleSource, ToAriadne};

/// This is a parser for JSON.
///
/// Parsing from str, returning [`Value`] and error with [`Rich`] type.
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
                if let Ok(i) = s.parse::<i64>() {
                    Value::Number(serde_json::Number::from(i))
                } else if let Ok(f) = s.parse::<f64>() {
                    Value::Number(
                        serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)),
                    )
                } else {
                    Value::Number(serde_json::Number::from(0))
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
            .collect::<Vec<_>>()
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

/// Implementation of `ToAriadne` for chumsky `Rich<char>` errors.
///
/// This allows chumsky parsing errors to be converted to beautiful ariadne reports
/// for display to users.
impl<'a> ToAriadne for Rich<'a, char> {
    fn to_report<'b>(
        &self,
        src: &SimpleSource<'b>,
    ) -> Report<'b, (String, std::ops::Range<usize>)> {
        let span = self.span();
        let message = self.to_string();
        let filename = src.name().to_string();
        let range = span.start..span.end;

        Report::build(ReportKind::Error, (filename.clone(), range.clone()))
            .with_message("JSON parsing error")
            .with_label(
                Label::new((filename, range))
                    .with_message(message)
                    .with_color(Color::Red),
            )
            .finish()
    }
}

/// Convenience function to emit chumsky parsing errors.
///
/// This function converts a list of chumsky `Rich<char>` errors to ariadne reports
/// and prints them to the console.
///
/// # Parameters
/// * `name` - Name of the source file
/// * `source` - Complete source text
/// * `errors` - List of chumsky parsing errors
pub fn emit_chumsky_errors<'a>(
    name: &'a str,
    source: &'a str,
    errors: impl IntoIterator<Item = &'a Rich<'a, char>>,
) {
    let simple = SimpleSource::new(name, source);
    let ariadne_source = ariadne::Source::from(source);

    for error in errors {
        let report = error.to_report(&simple);
        let _ = report.print((name.to_string(), ariadne_source.clone()));
    }
}
