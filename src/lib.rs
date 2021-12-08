pub mod lex;

use self::lex::LexError;

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ParseError {
    LexError(LexError),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::LexError(lex) => {
                write!(f, "lex error: {}", lex)
            }
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::LexError(lex) => Some(lex),
        }
    }
}

pub type Result<T> = std::result::Result<T, ParseError>;
