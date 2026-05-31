//! BMSON parser example using a chumsky-based custom JSON parser.
//!
//! This example demonstrates how to build a custom JSON parser with chumsky,
//! use it to parse BMSON data into a `serde_json::Value`, and then deserialize
//! that value into the `bms_rs::bmson::Bmson` struct.
//!
//! The parser supports error recovery (missing commas, trailing commas) and
//! provides detailed error diagnostics.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example bmson_chumsky_parser --features bmson -- path/to/file.bmson
//! ```

use std::env;
use std::fs;

use chumsky::{error::RichReason, prelude::*};
use serde_json::Value;

/// Parse a BMSON file from JSON string using the chumsky parser.
///
/// Returns the parsed `Bmson` value or an error message.
fn parse_bmson(json: &str) -> Result<bms_rs::bmson::Bmson<'_>, String> {
    let (value, parse_errors) = parser().parse(json.trim()).into_output_errors();

    let had_output = value.is_some();
    let (warnings, recovered, fatal) = split_chumsky_errors(parse_errors, had_output);

    // Print warnings
    for Warning(warning) in &warnings {
        eprintln!("Warning: {warning:?}");
    }

    // Print recovered errors
    for Recovered(error) in &recovered {
        eprintln!("Recovered error: {error:?}");
    }

    // If there are fatal errors and no output was produced, return error
    if !fatal.is_empty() && !had_output {
        for Error(error) in &fatal {
            eprintln!("Fatal error: {error:?}");
        }
        return Err("Failed to parse JSON.".to_string());
    }

    // Fall back to serde_json if chumsky didn't produce a value
    let json_value = match value {
        Some(v) => v,
        None => serde_json::from_str(json)
            .map_err(|e| format!("serde_json fallback also failed: {e}"))?,
    };

    // Deserialize into Bmson
    serde_json::from_value(json_value).map_err(|e| format!("Failed to deserialize BMSON: {e}"))
}

/// Chumsky-based JSON parser.
#[must_use]
fn parser<'a>() -> impl Parser<'a, &'a str, Value, extra::Err<Rich<'a, char>>> {
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
                    |hex_digits, e, emitter| {
                        let Ok(codepoint) = u32::from_str_radix(hex_digits, 16) else {
                            emitter.emit(Rich::custom(e.span(), "invalid unicode character"));
                            return '\u{FFFD}';
                        };
                        char::from_u32(codepoint).unwrap_or_else(|| {
                            emitter.emit(Rich::custom(e.span(), "invalid unicode character"));
                            '\u{FFFD}'
                        })
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
                for (key, json_value) in pairs {
                    map.insert(key, json_value);
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

/// Diagnostic warning intentionally emitted by the JSON parser using `Rich::custom`.
#[derive(Debug, Clone)]
struct Warning<'a>(Rich<'a, char>);

/// Error recovered by the JSON parser. These originated from grammar mismatches
/// that were recovered via `recover_with` or similar mechanisms.
#[derive(Debug, Clone)]
struct Recovered<'a>(Rich<'a, char>);

/// Unrecoverable JSON parsing error (no output value was produced).
#[derive(Debug, Clone)]
struct Error<'a>(Rich<'a, char>);

/// Split chumsky `Rich<char>` errors into `Warning`, `Recovered`, and `Error` buckets.
#[must_use]
fn split_chumsky_errors<'a>(
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
            RichReason::ExpectedFound { .. } if had_output => recovered.push(Recovered(err)),
            RichReason::ExpectedFound { .. } => fatal.push(Error(err)),
        }
    }
    (warnings, recovered, fatal)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let path = if let Some(p) = args.get(1) {
        p
    } else {
        eprintln!("Usage: bmson_chumsky_parser <path-to-bmson-file>");
        return;
    };

    let json = match fs::read_to_string(path) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("Failed to read file {path}: {e}");
            return;
        }
    };

    let bmson = match parse_bmson(&json) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    println!("Successfully parsed BMSON file: {path}");
    println!("Title: {}", bmson.info.title);
    println!("Artist: {}", bmson.info.artist);
    println!("Genre: {}", bmson.info.genre);
    println!("Level: {}", bmson.info.level);
    println!("BPM: {:.2}", bmson.info.init_bpm.as_f64());
    println!("Resolution: {}", bmson.info.resolution);
    println!("Sound channels: {}", bmson.sound_channels.len());
    println!("BPM events: {}", bmson.bpm_events.len());
    println!("Stop events: {}", bmson.stop_events.len());
}
