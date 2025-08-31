//! Parsing Bms from [TokenStream].
//!
//! Raw [String] == [lex] ==> [TokenStream] (in [BmsLexOutput]) == [parse] ==> [Bms] (in
//! BmsParseOutput)

pub mod check_playing;
pub mod model;
pub mod prompt;
pub mod validity;

use thiserror::Error;

use crate::bms::{
    ast::{
        AstBuildOutput, AstBuildWarningWithRange, AstParseOutput, AstParseWarningWithRange,
        AstRoot, rng::Rng,
    },
    command::{
        ObjId,
        channel::{
            Channel,
            mapper::{KeyLayoutBeat, KeyLayoutMapper},
        },
        mixin::SourceRangeMixin,
        time::{ObjTime, Track},
    },
    lex::token::TokenWithRange,
    prelude::SourceRangeMixinExt,
};

use self::{model::Bms, prompt::PromptHandler};

/// An error occurred when parsing the [`TokenStream`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ParseWarning {
    /// Syntax formed from the commands was invalid.
    #[error("syntax error: {0}")]
    SyntaxError(String),
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
pub type ParseWarningWithRange = SourceRangeMixin<ParseWarning>;

/// Bms Parse Output
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ParseOutput<T: KeyLayoutMapper = KeyLayoutBeat> {
    /// The output Bms.
    pub bms: Bms<T>,
    /// Warnings that occurred during parsing.
    pub parse_warnings: Vec<ParseWarningWithRange>,
}

impl<T: KeyLayoutMapper> Bms<T> {
    /// Parses a token stream into [`Bms`] without AST.
    pub fn from_token_stream<'a>(
        token_iter: impl IntoIterator<Item = &'a TokenWithRange<'a>>,
        mut prompt_handler: impl PromptHandler,
    ) -> ParseOutput<T> {
        let mut bms = Self::default();
        let mut parse_warnings = vec![];
        for token in token_iter {
            if let Err(error) = bms.parse(token, &mut prompt_handler) {
                parse_warnings.push(error.into_wrapper(token));
            }
        }

        ParseOutput {
            bms,
            parse_warnings,
        }
    }
}

/// Bms Parse Output with AST
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ParseOutputWithAst<T: KeyLayoutMapper = KeyLayoutBeat> {
    /// The output Bms.
    pub bms: Bms<T>,
    /// Warnings that occurred during AST building.
    pub ast_build_warnings: Vec<AstBuildWarningWithRange>,
    /// Warnings that occurred during AST parsing (RNG execution stage).
    pub ast_parse_warnings: Vec<AstParseWarningWithRange>,
    /// Warnings that occurred during parsing.
    pub parse_warnings: Vec<ParseWarningWithRange>,
}

impl<T: KeyLayoutMapper> Bms<T> {
    /// Parses a token stream into [`Bms`] with AST.
    pub fn from_token_stream_with_ast<'a>(
        token_iter: impl IntoIterator<Item = &'a TokenWithRange<'a>>,
        rng: impl Rng,
        prompt_handler: impl PromptHandler,
    ) -> ParseOutputWithAst<T> {
        let AstBuildOutput {
            root,
            ast_build_warnings,
        } = AstRoot::from_token_stream(token_iter);
        let (AstParseOutput { token_refs }, ast_parse_warnings) = root.parse_with_warnings(rng);
        let ParseOutput {
            bms,
            parse_warnings,
        } = Self::from_token_stream(token_refs, prompt_handler);
        ParseOutputWithAst {
            bms,
            ast_build_warnings,
            ast_parse_warnings,
            parse_warnings,
        }
    }
}
