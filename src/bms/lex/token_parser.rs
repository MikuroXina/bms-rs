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

/// Parser for common illegal but frequent mistakes in BMS sources.
///
/// This parser handles various common typos and formatting errors that are
/// frequently found in BMS files, providing more lenient parsing.
pub struct CommonRelaxer;

impl<'a> TokenParser<'a> for CommonRelaxer {
    fn try_parse(&self, cursor: &mut Cursor<'a>) -> Result<Option<Token<'a>>> {
        let checkpoint = cursor.save_checkpoint();

        let Some(command) = cursor.next_token() else {
            cursor.restore_checkpoint(checkpoint);
            return Ok(None);
        };

        // Handle common typos in random control commands
        let token = if command.eq_ignore_ascii_case("#RONDAM") {
            // #RONDAM n -> #RANDOM n
            let rand_max = cursor
                .next_token()
                .ok_or_else(|| cursor.make_err_expected_token("random max"))?
                .parse()
                .map_err(|_| cursor.make_err_expected_token("integer"))?;
            Some(Token::Random(rand_max))
        } else if command.eq_ignore_ascii_case("#END") {
            // Check for #END IF -> #ENDIF
            cursor
                .peek_next_token()
                .filter(|next_token| next_token.eq_ignore_ascii_case("IF"))
                .map(|_| {
                    cursor.next_token(); // consume "IF"
                    Token::EndIf
                })
        } else if command == "＃ENDIF" {
            // Full-width #ENDIF -> treat as #ENDIF
            Some(Token::EndIf)
        } else if let Some(n_part) = command
            .to_uppercase()
            .strip_prefix("#RANDOM")
            .filter(|_| command.len() > 7)
        {
            // #RANDOMn -> #RANDOM n
            n_part.parse().ok().map(Token::Random)
        } else if let Some(remaining) = command
            .to_uppercase()
            .strip_prefix("#IF")
            .filter(|_| command.len() > 3)
        {
            if remaining.chars().all(|c: char| c.is_ascii_digit()) {
                // #IFn -> #IF n
                remaining.parse().ok().map(Token::If)
            } else if remaining.eq_ignore_ascii_case("END") {
                // #IFEND -> #ENDIF
                Some(Token::EndIf)
            } else {
                None
            }
        } else {
            None
        };

        if token.is_none() {
            cursor.restore_checkpoint(checkpoint);
        }

        Ok(token)
    }
}

/// Creates a vector of default token parsers with common relaxer in the standard order.
pub fn default_parsers_with_relaxer<'a>() -> Vec<Box<dyn TokenParser<'a>>> {
    vec![
        Box::new(CommonRelaxer),
        Box::new(ControlFlowParser),
        Box::new(MessageParser),
        Box::new(HeaderParser),
        Box::new(CommentParser),
    ]
}
