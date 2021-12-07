pub mod command;
pub mod cursor;
pub mod token;

use self::{
    cursor::Cursor,
    token::{Token, TokenStream},
};

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ParseError {
    UnknownCommand {
        line: usize,
        col: usize,
    },
    ExpectedToken {
        line: usize,
        col: usize,
        message: &'static str,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnknownCommand { line, col } => {
                write!(f, "unknown command found at line {}, col {}", line, col)
            }
            ParseError::ExpectedToken { line, col, message } => write!(
                f,
                "expected {}, but not found at line {}, col {}",
                message, line, col
            ),
        }
    }
}

impl std::error::Error for ParseError {}

pub type Result<T> = std::result::Result<T, ParseError>;

pub fn parse(source: &str) -> Result<TokenStream> {
    let mut cursor = Cursor::new(source);

    let mut tokens = vec![];
    while !cursor.is_end() {
        tokens.push(Token::parse(&mut cursor)?);
    }
    Ok(TokenStream::from_tokens(tokens))
}
