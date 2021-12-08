pub mod lex;
pub mod parse;

use self::{lex::LexError, parse::ParseError};

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum BmsError {
    LexError(LexError),
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

impl std::fmt::Display for BmsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BmsError::LexError(lex) => {
                write!(f, "lex error: {}", lex)
            }
            BmsError::ParseError(parse) => {
                write!(f, "parse error: {}", parse)
            }
        }
    }
}

impl std::error::Error for BmsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BmsError::LexError(lex) => Some(lex),
            BmsError::ParseError(parse) => Some(parse),
        }
    }
}

pub type Result<T> = std::result::Result<T, BmsError>;
