pub mod lex;
pub mod parse;
pub mod time;

use thiserror::Error;

use self::{lex::LexError, parse::ParseError};

/// An error occurred when parsing the BMS format file.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum BmsError {
    /// An error comes from lexical analyzer.
    #[error("lex error: {0}")]
    LexError(LexError),
    /// An error comes from syntax parser.
    #[error("parse error: {0}")]
    ParseError(ParseError),
}

impl From<LexError> for BmsError {
    fn from(e: LexError) -> Self {
        Self::LexError(e)
    }
}
impl From<ParseError> for BmsError {
    fn from(e: ParseError) -> Self {
        Self::ParseError(e)
    }
}

/// A custom result type for bms-rs.
pub type Result<T> = std::result::Result<T, BmsError>;
