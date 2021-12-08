pub mod lex;

use self::lex::LexError;

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum BmsError {
    LexError(LexError),
}

impl From<LexError> for BmsError {
    fn from(e: LexError) -> Self {
        Self::LexError(e)
    }
}

impl std::fmt::Display for BmsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BmsError::LexError(lex) => {
                write!(f, "lex error: {}", lex)
            }
        }
    }
}

impl std::error::Error for BmsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BmsError::LexError(lex) => Some(lex),
        }
    }
}

pub type Result<T> = std::result::Result<T, BmsError>;
