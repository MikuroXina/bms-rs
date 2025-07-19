//! Parser for BMS format. The reason why the implementation separated into lex and parse is the score may contain some randomized elements such as `#RANDOM`. This separation make us able to parse the tokens with the custom random generator cheaply.

pub mod header;
pub mod notes;
pub mod obj;
pub mod prompt;
mod random;
pub mod rng;

use std::ops::{Deref, DerefMut};

use thiserror::Error;

use self::{
    header::Header, notes::Notes, prompt::PromptHandler, random::ControlFlowRule, rng::Rng,
};
use crate::bms::{
    lex::{command::ObjId, token::Token},
    parse::random::parse_control_flow,
};

use super::lex::BmsLexOutput;

/// An error occurred when parsing the [`TokenStream`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ParseWarning {
    /// Syntax formed from the commands was invalid.
    #[error("syntax error: {0}")]
    SyntaxError(String),
    /// Violation of control flow rule.
    #[error("violate control flow rule: {0}")]
    ViolateControlFlowRule(#[from] ControlFlowRule),
    /// The invalid real number for the BPM.
    #[error("not a number bpm: {0}")]
    BpmParseError(String),
    /// The object has required but not defined,
    #[error("undefined object: {0:?}")]
    UndefinedObject(ObjId),
    /// Parsing is halted because `prompt_handler` returned [`PromptChoice::Halt`].
    #[error("parsing is halted by prompt handler")]
    Halted,
}

/// type alias of core::result::Result<T, ParseWarning>
pub(crate) type Result<T> = core::result::Result<T, ParseWarning>;

/// A score data of BMS format.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bms {
    /// The header data in the score.
    pub header: Header,
    /// The objects in the score.
    pub notes: Notes,
    /// Lines that not starts with `'#'`.
    pub non_command_lines: Vec<String>,
    /// Lines that starts with `'#'`, but not recognized as vaild command.
    pub unknown_command_lines: Vec<String>,
}

/// Bms Parse Output
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BmsParseOutput {
    /// The output Bms.
    pub bms: Bms,
    /// Errors
    pub warnings: Vec<ParseWarning>,
}

/// The type of parsing tokens iter.
pub struct BmsParseTokenIter<'a>(std::iter::Peekable<std::slice::Iter<'a, Token<'a>>>);

impl<'a> BmsParseTokenIter<'a> {
    /// Create iter from BmsLexOutput reference.
    pub fn from_lex_output(value: &'a BmsLexOutput) -> Self {
        Self(value.tokens.iter().as_slice().iter().peekable())
    }
    /// Create iter from Token list reference.
    pub fn from_tokens(value: &'a [Token<'a>]) -> Self {
        Self(value.iter().peekable())
    }
}

impl<'a> From<&'a BmsLexOutput<'a>> for BmsParseTokenIter<'a> {
    fn from(value: &'a BmsLexOutput<'a>) -> Self {
        Self(value.tokens.iter().peekable())
    }
}

impl<'a, T: AsRef<[Token<'a>]> + ?Sized> From<&'a T> for BmsParseTokenIter<'a> {
    fn from(value: &'a T) -> Self {
        Self(value.as_ref().iter().peekable())
    }
}

impl<'a> Deref for BmsParseTokenIter<'a> {
    type Target = std::iter::Peekable<std::slice::Iter<'a, Token<'a>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for BmsParseTokenIter<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Bms {
    /// Parses a token stream into [`Bms`] with a random generator [`Rng`].
    pub fn from_token_stream<'a>(
        token_iter: impl Into<BmsParseTokenIter<'a>>,
        rng: impl Rng,
        mut prompt_handler: impl PromptHandler,
    ) -> BmsParseOutput {
        let (continue_tokens, mut errors) = parse_control_flow(&mut token_iter.into(), rng);
        let mut notes = Notes::default();
        let mut header = Header::default();
        let mut non_command_lines: Vec<String> = Vec::new();
        let mut unknown_command_lines: Vec<String> = Vec::new();
        for &token in continue_tokens.iter() {
            if let Err(error) = notes.parse(token, &header) {
                errors.push(error);
            }
            if let Err(error) = header.parse(token, &mut prompt_handler) {
                errors.push(error);
            }
            match token {
                Token::NotACommand(comment) => non_command_lines.push(comment.to_string()),
                Token::UnknownCommand(comment) => unknown_command_lines.push(comment.to_string()),
                _ => (),
            }
        }
        BmsParseOutput {
            bms: Self {
                header,
                notes,
                non_command_lines,
                unknown_command_lines,
            },
            warnings: errors,
        }
    }
}
