//! Modular parsers for BMS lexical analysis.
//!
//! This module contains specialized parsers that work with `&mut Cursor` to parse
//! different types of BMS tokens. Each parser is responsible for a specific token type
//! and leverages `Cursor::try_next_token` for error recovery and backtracking.

use num::BigUint;
use std::borrow::Cow;

use crate::bms::{command::time::Track, prelude::read_channel};

use super::{Result, cursor::Cursor, token::Token};

/// Trait for parsers that can parse tokens from a cursor.
pub trait TokenParser<'a> {
    /// Attempts to parse a token from the cursor.
    /// Returns `Some(Ok(token))` if parsing succeeds,
    /// `None` if this parser doesn't handle the current input,
    /// or `Some(Err(warning))` if parsing fails.
    fn try_parse(&self, cursor: &mut Cursor<'a>) -> Option<Result<Token<'a>>>;
}

/// Parser for control flow commands like #RANDOM, #IF, #SWITCH, etc.
pub struct ControlFlowParser;

impl<'a> TokenParser<'a> for ControlFlowParser {
    fn try_parse(&self, cursor: &mut Cursor<'a>) -> Option<Result<Token<'a>>> {
        enum Kind {
            Random,
            SetRandom,
            If,
            ElseIf,
            Else,
            EndIf,
            EndRandom,
            Switch,
            SetSwitch,
            Case,
            Skip,
            Def,
            EndSwitch,
        }

        let kind = match cursor.try_next_token(|command| {
            let k = match command.to_uppercase().as_str() {
                "#RANDOM" => Some(Kind::Random),
                "#SETRANDOM" => Some(Kind::SetRandom),
                "#IF" => Some(Kind::If),
                "#ELSEIF" => Some(Kind::ElseIf),
                "#ELSE" => Some(Kind::Else),
                "#ENDIF" => Some(Kind::EndIf),
                "#ENDRANDOM" => Some(Kind::EndRandom),
                "#SWITCH" => Some(Kind::Switch),
                "#SETSWITCH" => Some(Kind::SetSwitch),
                "#CASE" => Some(Kind::Case),
                "#SKIP" => Some(Kind::Skip),
                "#DEF" => Some(Kind::Def),
                "#ENDSW" => Some(Kind::EndSwitch),
                _ => None,
            };
            Ok(k)
        }) {
            Ok(kind) => kind,
            Err(e) => return Some(Err(e)),
        };

        let kind = kind?;

        let token = match kind {
            Kind::Random => {
                let rand_max = match cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("random max"))
                    .and_then(|s| {
                        s.parse()
                            .map_err(|_| cursor.make_err_expected_token("integer"))
                    }) {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };
                Token::Random(rand_max)
            }
            Kind::SetRandom => {
                let rand_value = match cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("random value"))
                    .and_then(|s| {
                        s.parse()
                            .map_err(|_| cursor.make_err_expected_token("integer"))
                    }) {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };
                Token::SetRandom(rand_value)
            }
            Kind::If => {
                let rand_target = match cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("random target"))
                    .and_then(|s| {
                        s.parse()
                            .map_err(|_| cursor.make_err_expected_token("integer"))
                    }) {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };
                Token::If(rand_target)
            }
            Kind::ElseIf => {
                let rand_target = match cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("random target"))
                    .and_then(|s| {
                        s.parse()
                            .map_err(|_| cursor.make_err_expected_token("integer"))
                    }) {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };
                Token::ElseIf(rand_target)
            }
            Kind::Else => Token::Else,
            Kind::EndIf => Token::EndIf,
            Kind::EndRandom => Token::EndRandom,
            Kind::Switch => {
                let switch_max = match cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("switch max"))
                    .and_then(|s| {
                        s.parse()
                            .map_err(|_| cursor.make_err_expected_token("integer"))
                    }) {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };
                Token::Switch(switch_max)
            }
            Kind::SetSwitch => {
                let switch_value = match cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("switch value"))
                    .and_then(|s| {
                        s.parse()
                            .map_err(|_| cursor.make_err_expected_token("integer"))
                    }) {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };
                Token::SetSwitch(switch_value)
            }
            Kind::Case => {
                let case_value = match cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("switch case value"))
                    .and_then(|s| {
                        s.parse()
                            .map_err(|_| cursor.make_err_expected_token("integer"))
                    }) {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };
                Token::Case(case_value)
            }
            Kind::Skip => Token::Skip,
            Kind::Def => Token::Def,
            Kind::EndSwitch => Token::EndSwitch,
        };

        Some(Ok(token))
    }
}

/// Parser for message commands in format #XXXYY:ZZ...
pub struct MessageParser;

impl<'a> TokenParser<'a> for MessageParser {
    fn try_parse(&self, cursor: &mut Cursor<'a>) -> Option<Result<Token<'a>>> {
        let matched = match cursor.try_next_token(|command| {
            if !command.starts_with('#') || command.chars().nth(6) != Some(':') || command.len() < 8
            {
                return Ok(None);
            }
            Ok(Some(()))
        }) {
            Ok(matched) => matched,
            Err(e) => return Some(Err(e)),
        };

        matched?;

        let message_line = cursor.next_line_entire().trim_start();
        let track = match message_line[1..4]
            .parse()
            .map_err(|_| cursor.make_err_expected_token("[000-999]"))
        {
            Ok(track) => track,
            Err(e) => return Some(Err(e)),
        };
        let channel = &message_line[4..6];
        let message = &message_line[7..];

        let channel = match read_channel(channel)
            .ok_or_else(|| cursor.make_err_unknown_channel(channel.to_string()))
        {
            Ok(channel) => channel,
            Err(e) => return Some(Err(e)),
        };

        Some(Ok(Token::Message {
            track: Track(track),
            channel,
            message: Cow::Borrowed(message),
        }))
    }
}

/// Parser for header commands (other # commands).
pub struct HeaderParser;

impl<'a> TokenParser<'a> for HeaderParser {
    fn try_parse(&self, cursor: &mut Cursor<'a>) -> Option<Result<Token<'a>>> {
        let matched = match cursor.try_next_token(|command| {
            if !command.starts_with('#') {
                return Ok(None);
            }
            Ok(Some(command))
        }) {
            Ok(matched) => matched,
            Err(e) => return Some(Err(e)),
        };

        let command = matched?;

        let args = cursor.next_line_remaining();
        Some(Ok(Token::Header {
            name: command.trim_start_matches('#').to_owned().into(),
            args: args.into(),
        }))
    }
}

/// Parser for non-command lines (comments).
pub struct CommentParser;

impl<'a> TokenParser<'a> for CommentParser {
    fn try_parse(&self, cursor: &mut Cursor<'a>) -> Option<Result<Token<'a>>> {
        let matched = match cursor.try_next_token(|command| {
            if command.starts_with('#') {
                return Ok(None);
            }
            Ok(Some(()))
        }) {
            Ok(matched) => matched,
            Err(e) => return Some(Err(e)),
        };

        matched?;

        let comment = cursor.next_line_entire();
        Some(Ok(Token::NotACommand(comment)))
    }
}

/// Creates a vector of default token parsers in the standard order.
#[must_use]
pub fn default_parsers<'a>() -> Vec<Box<dyn TokenParser<'a>>> {
    vec![
        Box::new(ControlFlowParser),
        Box::new(MessageParser),
        Box::new(HeaderParser),
        Box::new(CommentParser),
    ]
}

/// Parser for common illegal but frequent mistakes in BMS sources.
///
/// This parser handles various common typos and formatting errors that are
/// frequently found in BMS files, providing more lenient parsing.
pub struct CommonRelaxer;

impl<'a> TokenParser<'a> for CommonRelaxer {
    fn try_parse(&self, cursor: &mut Cursor<'a>) -> Option<Result<Token<'a>>> {
        enum RelaxAction {
            RandomFromNext,
            EndIfDirect,
            RandomFromSuffix(BigUint),
            IfFromSuffix(BigUint),
        }

        let action = match cursor.try_next_token(|command| {
            let upper = command.to_uppercase();
            let act = if upper == "#RONDAM" {
                Some(RelaxAction::RandomFromNext)
            } else if command == "＃ENDIF" {
                Some(RelaxAction::EndIfDirect)
            } else if let Some(n_part) = upper.strip_prefix("#RANDOM").filter(|_| command.len() > 7)
            {
                if let Ok(n) = n_part.parse::<BigUint>() {
                    Some(RelaxAction::RandomFromSuffix(n))
                } else {
                    None
                }
            } else if let Some(remaining) = upper.strip_prefix("#IF").filter(|_| command.len() > 3)
            {
                if remaining.chars().all(|c: char| c.is_ascii_digit()) {
                    remaining
                        .parse::<BigUint>()
                        .ok()
                        .map(RelaxAction::IfFromSuffix)
                } else if remaining.eq_ignore_ascii_case("END") {
                    Some(RelaxAction::EndIfDirect)
                } else {
                    None
                }
            } else {
                None
            };
            Ok(act)
        }) {
            Ok(action) => action,
            Err(e) => return Some(Err(e)),
        };

        let action = action?;

        let token = match action {
            RelaxAction::RandomFromNext => {
                let rand_max = match cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("random max"))
                    .and_then(|s| {
                        s.parse::<BigUint>()
                            .map_err(|_| cursor.make_err_expected_token("integer"))
                    }) {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };
                Token::Random(rand_max)
            }
            RelaxAction::EndIfDirect => Token::EndIf,
            RelaxAction::RandomFromSuffix(n) => Token::Random(n),
            RelaxAction::IfFromSuffix(n) => Token::If(n),
        };

        Some(Ok(token))
    }
}

/// Creates a vector of default token parsers with common relaxer in the standard order.
#[must_use]
pub fn default_parsers_with_relaxer<'a>() -> Vec<Box<dyn TokenParser<'a>>> {
    vec![
        Box::new(CommonRelaxer),
        Box::new(ControlFlowParser),
        Box::new(MessageParser),
        Box::new(HeaderParser),
        Box::new(CommentParser),
    ]
}
