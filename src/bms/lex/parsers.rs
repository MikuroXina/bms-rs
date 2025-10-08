//! Modular parsers for BMS lexical analysis.
//!
//! This module contains specialized parsers that work with `&mut Cursor` to parse
//! different types of BMS tokens. Each parser is responsible for a specific token type
//! and uses `CursorCheckpoint` for error recovery and backtracking.

use std::borrow::Cow;

use crate::bms::{
    command::{mixin::SourceRangeMixin, time::Track},
    prelude::read_channel,
};

use super::{
    Result,
    cursor::Cursor,
    token::{Token, TokenWithRange},
};

/// Trait for parsers that can parse tokens from a cursor.
pub trait TokenParser<'a> {
    /// Attempts to parse a token from the cursor.
    /// Returns `Ok(Some(token))` if parsing succeeds,
    /// `Ok(None)` if this parser doesn't handle the current input,
    /// or `Err(warning)` if parsing fails.
    fn try_parse(&self, cursor: &mut Cursor<'a>) -> Result<Option<Token<'a>>>;
}

/// Parser for control flow commands like #RANDOM, #IF, #SWITCH, etc.
pub struct ControlFlowParser;

impl<'a> TokenParser<'a> for ControlFlowParser {
    fn try_parse(&self, cursor: &mut Cursor<'a>) -> Result<Option<Token<'a>>> {
        let checkpoint = cursor.save_checkpoint();

        let Some(command) = cursor.next_token() else {
            cursor.restore_checkpoint(checkpoint);
            return Ok(None);
        };

        let token = match command.to_uppercase().as_str() {
            "#RANDOM" => {
                let rand_max = cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("random max"))?
                    .parse()
                    .map_err(|_| cursor.make_err_expected_token("integer"))?;
                Some(Token::Random(rand_max))
            }
            "#SETRANDOM" => {
                let rand_value = cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("random value"))?
                    .parse()
                    .map_err(|_| cursor.make_err_expected_token("integer"))?;
                Some(Token::SetRandom(rand_value))
            }
            "#IF" => {
                let rand_target = cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("random target"))?
                    .parse()
                    .map_err(|_| cursor.make_err_expected_token("integer"))?;
                Some(Token::If(rand_target))
            }
            "#ELSEIF" => {
                let rand_target = cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("random target"))?
                    .parse()
                    .map_err(|_| cursor.make_err_expected_token("integer"))?;
                Some(Token::ElseIf(rand_target))
            }
            "#ELSE" => Some(Token::Else),
            "#ENDIF" => Some(Token::EndIf),
            "#ENDRANDOM" => Some(Token::EndRandom),
            "#SWITCH" => {
                let switch_max = cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("switch max"))?
                    .parse()
                    .map_err(|_| cursor.make_err_expected_token("integer"))?;
                Some(Token::Switch(switch_max))
            }
            "#SETSWITCH" => {
                let switch_value = cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("switch value"))?
                    .parse()
                    .map_err(|_| cursor.make_err_expected_token("integer"))?;
                Some(Token::SetSwitch(switch_value))
            }
            "#CASE" => {
                let case_value = cursor
                    .next_token()
                    .ok_or_else(|| cursor.make_err_expected_token("switch case value"))?
                    .parse()
                    .map_err(|_| cursor.make_err_expected_token("integer"))?;
                Some(Token::Case(case_value))
            }
            "#SKIP" => Some(Token::Skip),
            "#DEF" => Some(Token::Def),
            "#ENDSW" => Some(Token::EndSwitch),
            _ => None,
        };

        if token.is_none() {
            cursor.restore_checkpoint(checkpoint);
        }

        Ok(token)
    }
}

/// Parser for message commands in format #XXXYY:ZZ...
pub struct MessageParser;

impl<'a> TokenParser<'a> for MessageParser {
    fn try_parse(&self, cursor: &mut Cursor<'a>) -> Result<Option<Token<'a>>> {
        let checkpoint = cursor.save_checkpoint();

        let Some(command) = cursor.next_token() else {
            cursor.restore_checkpoint(checkpoint);
            return Ok(None);
        };

        if !command.starts_with('#') || command.chars().nth(6) != Some(':') || command.len() < 8 {
            cursor.restore_checkpoint(checkpoint);
            return Ok(None);
        }

        let message_line = cursor.next_line_entire().trim_start();
        let track = message_line[1..4]
            .parse()
            .map_err(|_| cursor.make_err_expected_token("[000-999]"))?;
        let channel = &message_line[4..6];
        let message = &message_line[7..];

        let channel = read_channel(channel)
            .ok_or_else(|| cursor.make_err_unknown_channel(channel.to_string()))?;

        Ok(Some(Token::Message {
            track: Track(track),
            channel,
            message: Cow::Borrowed(message),
        }))
    }
}

/// Parser for header commands (other # commands).
pub struct HeaderParser;

impl<'a> TokenParser<'a> for HeaderParser {
    fn try_parse(&self, cursor: &mut Cursor<'a>) -> Result<Option<Token<'a>>> {
        let checkpoint = cursor.save_checkpoint();

        let Some(command) = cursor.next_token() else {
            cursor.restore_checkpoint(checkpoint);
            return Ok(None);
        };

        if !command.starts_with('#') {
            cursor.restore_checkpoint(checkpoint);
            return Ok(None);
        }

        let args = cursor.next_line_remaining();
        Ok(Some(Token::Header {
            name: command.trim_start_matches('#').to_owned().into(),
            args: args.into(),
        }))
    }
}

/// Parser for non-command lines (comments).
pub struct CommentParser;

impl<'a> TokenParser<'a> for CommentParser {
    fn try_parse(&self, cursor: &mut Cursor<'a>) -> Result<Option<Token<'a>>> {
        let checkpoint = cursor.save_checkpoint();

        let Some(command) = cursor.next_token() else {
            cursor.restore_checkpoint(checkpoint);
            return Ok(None);
        };

        if command.starts_with('#') {
            cursor.restore_checkpoint(checkpoint);
            return Ok(None);
        }

        let comment = cursor.next_line_entire();
        Ok(Some(Token::NotACommand(comment)))
    }
}

/// Main parser that coordinates all specialized parsers.
pub struct LexerParser;

impl LexerParser {
    /// Creates a new lexer parser.
    pub fn new() -> Self {
        Self
    }

    /// Parses a token from the cursor using all available parsers.
    pub fn parse_token<'a>(&self, cursor: &mut Cursor<'a>) -> Result<TokenWithRange<'a>> {
        let command_range = cursor.save_checkpoint();

        // Try each parser in order
        let parsers: Vec<Box<dyn TokenParser<'a>>> = vec![
            Box::new(ControlFlowParser),
            Box::new(MessageParser),
            Box::new(HeaderParser),
            Box::new(CommentParser),
        ];

        for parser in parsers {
            let checkpoint = cursor.save_checkpoint();

            match parser.try_parse(cursor) {
                Ok(Some(token)) => {
                    let token_range = command_range.index..cursor.index();
                    return Ok(SourceRangeMixin::new(token, token_range));
                }
                Ok(None) => {
                    cursor.restore_checkpoint(checkpoint);
                    continue;
                }
                Err(warning) => {
                    cursor.restore_checkpoint(checkpoint);
                    return Err(warning);
                }
            }
        }

        Err(cursor.make_err_expected_token("valid token"))
    }
}

impl Default for LexerParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a vector of default token parsers in the standard order.
pub fn default_parsers<'a>() -> Vec<Box<dyn TokenParser<'a>>> {
    vec![
        Box::new(ControlFlowParser),
        Box::new(MessageParser),
        Box::new(HeaderParser),
        Box::new(CommentParser),
    ]
}
