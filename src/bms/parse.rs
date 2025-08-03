//! Parsing Bms from [TokenStream].
//!
//! Raw [String] == [lex] ==> [TokenStream] (in [BmsLexOutput]) == [parse] ==> [Bms] (in
//! BmsParseOutput)

pub mod check_playing;
pub mod model;
pub mod prompt;
pub mod random;

use std::ops::{Deref, DerefMut};

use thiserror::Error;

use crate::bms::{command::ObjId, lex::token::Token, parse::random::parse_control_flow};

use self::{
    check_playing::{PlayingError, PlayingWarning},
    model::Bms,
    prompt::PromptHandler,
    random::{ControlFlowRule, rng::Rng},
};
use super::lex::BmsLexOutput;

/// An error occurred when parsing the [`TokenStream`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ParseWarningContent {
    /// Syntax formed from the commands was invalid.
    #[error("syntax error: {0}")]
    SyntaxError(String),
    /// Violation of control flow rule.
    #[error("violate control flow rule: {0}")]
    ViolateControlFlowRule(#[from] ControlFlowRule),
    /// The object has required but not defined,
    #[error("undefined object: {0:?}")]
    UndefinedObject(ObjId),
    /// Parsing is warned because `prompt_handler` returned [`DuplicationWorkaround::Warn`].
    #[error("parsing is warned by prompt handler")]
    PromptHandlerWarning,
}

/// type alias of core::result::Result<T, ParseWarningContent>
pub(crate) type Result<T> = core::result::Result<T, ParseWarningContent>;

/// A parse warning with position information.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ParseWarning {
    /// The content of the parse warning.
    pub content: ParseWarningContent,
    /// The row (line number) where the warning occurred.
    pub row: usize,
    /// The column (character position) where the warning occurred.
    pub col: usize,
}

impl std::fmt::Display for ParseWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at line {}, column {}", self.content, self.row, self.col)
    }
}

impl std::error::Error for ParseWarning {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.content)
    }
}

/// Bms Parse Output
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BmsParseOutput {
    /// The output Bms.
    pub bms: Bms,
    /// Warnings that occurred during parsing.
    pub parse_warnings: Vec<ParseWarning>,
    /// Warnings that occurred during playing.
    pub playing_warnings: Vec<PlayingWarning>,
    /// Errors that occurred during playing.
    pub playing_errors: Vec<PlayingError>,
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
        let (continue_tokens, mut parse_warnings) = parse_control_flow(&mut token_iter.into(), rng);
        let mut bms = Bms::default();
        for &token in continue_tokens.iter() {
            if let Err(error) = bms.parse(token, &mut prompt_handler) {
                parse_warnings.push(ParseWarning {
                    content: error,
                    row: token.row,
                    col: token.col,
                });
            }
        }

        let (playing_warnings, playing_errors) = bms.check_playing();

        BmsParseOutput {
            bms,
            parse_warnings,
            playing_warnings,
            playing_errors,
        }
    }
}
