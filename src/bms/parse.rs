//! BMS: Parse Module.
//!
//! Lex => [Parse]

pub mod model;
pub mod prompt;
pub mod random;

use std::ops::{Deref, DerefMut};

use thiserror::Error;

use crate::bms::{
    command::{NoteKind, ObjId},
    lex::token::Token,
    parse::random::parse_control_flow,
};

use self::{
    model::{Bms, Header, Notes},
    prompt::PromptHandler,
    random::{ControlFlowRule, rng::Rng},
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
    /// Parsing is warned because `prompt_handler` returned [`DuplicationWorkaround::Warn`].
    #[error("parsing is warned by prompt handler")]
    PromptHandlerWarning,
}

/// type alias of core::result::Result<T, ParseWarning>
pub(crate) type Result<T> = core::result::Result<T, ParseWarning>;

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
    /// Check for playing warnings and errors based on the parsed BMS data.
    fn check_playing_conditions(&self) -> (Vec<PlayingWarning>, Vec<PlayingError>) {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // Check for TotalUndefined warning
        if self.header.total.is_none() {
            warnings.push(PlayingWarning::TotalUndefined);
        }

        // Check for BPM-related conditions
        let has_bpm = self.header.bpm.is_some() || !self.header.bpm_changes.is_empty();
        if !has_bpm {
            errors.push(PlayingError::BpmUndefined);
        } else if self.header.bpm.is_none() {
            warnings.push(PlayingWarning::StartBpmUndefined);
        }

        // Check for notes
        if self.notes.all_notes().next().is_none() {
            errors.push(PlayingError::NoNotes);
        } else {
            // Check for displayable notes (Visible, Long, Landmine)
            let has_displayable = self.notes.all_notes().any(|note| {
                matches!(
                    note.kind,
                    NoteKind::Visible | NoteKind::Long | NoteKind::Landmine
                )
            });
            if !has_displayable {
                warnings.push(PlayingWarning::NoDisplayableNotes);
            }

            // Check for playable notes (all except Invisible)
            let has_playable = self.notes.all_notes().any(|note| note.kind.is_playable());
            if !has_playable {
                warnings.push(PlayingWarning::NoPlayableNotes);
            }
        }

        (warnings, errors)
    }

    /// Parses a token stream into [`Bms`] with a random generator [`Rng`].
    pub fn from_token_stream<'a>(
        token_iter: impl Into<BmsParseTokenIter<'a>>,
        rng: impl Rng,
        mut prompt_handler: impl PromptHandler,
    ) -> BmsParseOutput {
        let (continue_tokens, mut parse_warnings) = parse_control_flow(&mut token_iter.into(), rng);
        let mut notes = Notes::default();
        let mut header = Header::default();
        let mut non_command_lines: Vec<String> = Vec::new();
        let mut unknown_command_lines: Vec<String> = Vec::new();
        for &token in continue_tokens.iter() {
            if let Err(error) = notes.parse(token, &header) {
                parse_warnings.push(error);
            }
            if let Err(error) = header.parse(token, &mut prompt_handler) {
                parse_warnings.push(error);
            }
            match token {
                Token::NotACommand(comment) => non_command_lines.push(comment.to_string()),
                Token::UnknownCommand(comment) => unknown_command_lines.push(comment.to_string()),
                _ => (),
            }
        }
        let bms = Self {
            header,
            notes,
            non_command_lines,
            unknown_command_lines,
        };

        let (playing_warnings, playing_errors) = bms.check_playing_conditions();

        BmsParseOutput {
            bms,
            parse_warnings,
            playing_warnings,
            playing_errors,
        }
    }
}

/// Simpifies the warnings for playing, which would not make this chart unplayable.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlayingWarning {
    /// The `#TOTAL` is not specified.
    #[error("The `#TOTAL` is not specified.")]
    TotalUndefined,
    /// There is no displayable notes.
    #[error("There is no displayable notes.")]
    NoDisplayableNotes,
    /// There is no playable notes.
    #[error("There is no playable notes.")]
    NoPlayableNotes,
    /// The `#BPM` is not specified. If there are other bpm changes, the first one will be used.
    /// If there are no bpm changes, there will be an [`PlayingError::BpmUndefined`].
    #[error(
        "The `#BPM` is not specified. If there are other bpm changes, the first one will be used. If there are no bpm changes, there will be an [`PlayingError::BpmUndefined`]."
    )]
    StartBpmUndefined,
}

/// Simpifies the warnings for playing, which will make this chart unplayable.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlayingError {
    /// There is no bpm defined.
    #[error("There is no bpm defined.")]
    BpmUndefined,
    /// There is no notes.
    #[error("There is no notes.")]
    NoNotes,
}
