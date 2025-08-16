//! Parsing Bms from [TokenStream].
//!
//! Raw [String] == [lex] ==> [TokenStream] (in [BmsLexOutput]) == [parse] ==> [Bms] (in
//! BmsParseOutput)

pub mod check_playing;
pub mod model;
pub mod prompt;

use thiserror::Error;

use crate::bms::{
    ast::{ControlFlowRule, parse_control_flow, rng::Rng},
    command::{
        ObjId,
        channel::Channel,
        mixin::{SourcePosMixin, SourcePosMixinExt},
        time::{ObjTime, Track},
    },
    lex::TokenIter,
};

use self::{
    check_playing::{PlayingError, PlayingWarning},
    model::Bms,
    prompt::PromptHandler,
};

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
    /// The object has required but not defined,
    #[error("undefined object: {0:?}")]
    UndefinedObject(ObjId),
    /// Has duplicated definition, that `prompt_handler` returned [`DuplicationWorkaround::Warn`].
    #[error("duplicating definition: {0}")]
    DuplicatingDef(ObjId),
    /// Has duplicated track object, that `prompt_handler` returned [`DuplicationWorkaround::Warn`].
    #[error("duplicating track object: {0} {1}")]
    DuplicatingTrackObj(Track, Channel),
    /// Has duplicated channel object, that `prompt_handler` returned [`DuplicationWorkaround::Warn`].
    #[error("duplicating channel object: {0} {1}")]
    DuplicatingChannelObj(ObjTime, Channel),
    /// Unexpected control flow.
    #[error("unexpected control flow")]
    UnexpectedControlFlow,
}

impl SourcePosMixinExt for ParseWarning {}

/// type alias of core::result::Result<T, ParseWarning>
pub(crate) type Result<T> = core::result::Result<T, ParseWarning>;

/// A parse warning with position information.
pub type ParseWarningWithPos = SourcePosMixin<ParseWarning>;

/// Bms Parse Output
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BmsParseOutput {
    /// The output Bms.
    pub bms: Bms,
    /// Warnings that occurred during parsing.
    pub parse_warnings: Vec<ParseWarningWithPos>,
    /// Warnings that occurred during playing.
    pub playing_warnings: Vec<PlayingWarning>,
    /// Errors that occurred during playing.
    pub playing_errors: Vec<PlayingError>,
}

impl Bms {
    /// Parses a token stream into [`Bms`] with a random generator [`Rng`].
    pub fn from_token_stream<'a>(
        token_iter: impl Into<TokenIter<'a>>,
        rng: impl Rng,
        mut prompt_handler: impl PromptHandler,
    ) -> BmsParseOutput {
        let (continue_tokens, mut parse_warnings) = parse_control_flow(&mut token_iter.into(), rng);
        let mut bms = Bms::default();
        for &token in continue_tokens.iter() {
            if let Err(error) = bms.parse(token, &mut prompt_handler) {
                parse_warnings.push(error.into_wrapper(token));
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
