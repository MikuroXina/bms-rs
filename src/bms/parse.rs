//! Parsing Bms from [TokenStream].
//!
//! Raw [String] == [lex] ==> [TokenStream] (in [BmsLexOutput]) == [parse] ==> [Bms] (in
//! BmsParseOutput)

pub mod check_playing;
pub mod model;
pub mod prompt;

use crate::bms::command::{PositionWrapper, PositionWrapperExt};
use thiserror::Error;

use crate::bms::{BmsTokenIter, command::ObjId};

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
    /// Parsing is warned because `prompt_handler` returned [`DuplicationWorkaround::Warn`].
    #[error("has duplication in defines, or same event in the same time")]
    HasDuplication,
    /// Unexpected Token like control flow tokens.
    #[error("has unexpected token, e.g. control flow tokens")]
    UnexpectedToken,
}

impl PositionWrapperExt for ParseWarning {}

/// type alias of core::result::Result<T, ParseWarning>
pub(crate) type Result<T> = core::result::Result<T, ParseWarning>;

// `ParseWarning` 类型别名已删除，请直接使用 `PositionWrapper<ParseWarning>`。

/// Bms Parse Output
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BmsParseOutput {
    /// The output Bms.
    pub bms: Bms,
    /// Warnings that occurred during parsing.
    pub parse_warnings: Vec<PositionWrapper<ParseWarning>>,
}

impl Bms {
    /// Parses a token stream into [`Bms`] with a random generator [`Rng`].
    pub fn from_token_stream<'a>(
        token_iter: impl Into<BmsTokenIter<'a>>,
        mut prompt_handler: impl PromptHandler,
    ) -> BmsParseOutput {
        let mut bms = Bms::default();
        let token_iter: BmsTokenIter<'a> = token_iter.into();
        let mut parse_warnings = vec![];
        for token in token_iter.0 {
            if let Err(error) = bms.parse(token, &mut prompt_handler) {
                parse_warnings.push(error.into_wrapper_manual(token.row, token.column));
            }
        }

        BmsParseOutput {
            bms,
            parse_warnings,
        }
    }
}
