//! Parsing Bms from [TokenStream].
//!
//! Raw [String] == [lex] ==> [TokenStream] (in [BmsLexOutput]) == [parse] ==> [Bms] (in
//! BmsParseOutput)

pub mod check_playing;
pub mod model;
pub mod prompt;

use thiserror::Error;

use crate::bms::{BmsTokenIter, command::ObjId};

use self::{model::Bms, prompt::PromptHandler};

/// An error occurred when parsing the [`TokenStream`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ParseWarningContent {
    /// Syntax formed from the commands was invalid.
    #[error("syntax error: {0}")]
    SyntaxError(String),
    /// The object has required but not defined,
    #[error("undefined object: {0:?}")]
    UndefinedObject(ObjId),
    /// Parsing is warned because `prompt_handler` returned [`DuplicationWorkaround::Warn`].
    #[error("parsing is warned by prompt handler")]
    HasDuplication,
}

/// type alias of core::result::Result<T, ParseWarningContent>
pub(crate) type Result<T> = core::result::Result<T, ParseWarningContent>;

/// A parse warning with position information.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ParseWarning {
    /// The content of the parse warning.
    #[source]
    pub content: ParseWarningContent,
    /// The row (line number) where the warning occurred.
    pub row: usize,
    /// The column (character position) where the warning occurred.
    pub col: usize,
}

impl std::fmt::Display for ParseWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at line {}, column {}",
            self.content, self.row, self.col
        )
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
                parse_warnings.push(ParseWarning {
                    content: error,
                    row: token.row,
                    col: token.col,
                });
            }
        }

        BmsParseOutput {
            bms,
            parse_warnings,
        }
    }
}
