//! Parsing Bms from [TokenStream].
//!
//! Raw [String] == [lex] ==> [TokenStream] (in [BmsLexOutput]) == [parse] ==> [Bms] (in
//! BmsParseOutput)

pub mod check_playing;
pub mod model;
pub mod prompt;

use thiserror::Error;

use crate::bms::{
    ast::{AstBuildOutput, AstBuildWarning, AstParseOutput, AstRoot, rng::Rng},
    command::{
        ObjId,
        channel::Channel,
        mixin::SourcePosMixin,
        time::{ObjTime, Track},
    },
    lex::token::TokenWithPos,
    prelude::SourcePosMixinExt,
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
    AstBuild(#[from] AstBuildWarning),
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
    pub fn from_token_stream_with_ast<'a>(
        token_iter: impl IntoIterator<Item = &'a TokenWithPos<'a>>,
        rng: impl Rng,
        prompt_handler: impl PromptHandler,
    ) -> BmsParseOutput {
        let AstBuildOutput {
            root,
            ast_build_warnings,
        } = AstRoot::from_token_stream(token_iter);
        let AstParseOutput { token_refs: tokens } = root.parse(rng);
        // Build Bms without AST.
        let BmsParseOutput {
            bms,
            parse_warnings,
            playing_warnings,
            playing_errors,
        } = Bms::from_token_stream(tokens.iter().cloned(), prompt_handler);
        let new_parse_warnings = ast_build_warnings
            .into_iter()
            .map(|w| {
                let (content, r, c) = w.into();
                ParseWarning::AstBuild(content).into_wrapper_manual(r, c)
            })
            .chain(parse_warnings)
            .collect();
        BmsParseOutput {
            bms,
            parse_warnings: new_parse_warnings,
            playing_warnings,
            playing_errors,
        }
    }

    /// Parses a token stream into [`Bms`] without AST.
    pub fn from_token_stream<'a>(
        token_iter: impl IntoIterator<Item = &'a TokenWithPos<'a>>,
        mut prompt_handler: impl PromptHandler,
    ) -> BmsParseOutput {
        let mut bms = Bms::default();
        let mut parse_warnings = vec![];
        for token in token_iter {
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
