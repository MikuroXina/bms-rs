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

pub type Result<T> = std::result::Result<T, ParseError>;

pub fn parse(source: &str) -> Result<TokenStream> {
    let mut cursor = Cursor::new(source);

    let tokens = std::iter::repeat_with(move || Token::parse(&mut cursor)).fold(
        Ok(vec![]),
        |mut tokens, token| {
            if let Ok(tokens) = &mut tokens {
                tokens.push(token?);
            }
            tokens
        },
    )?;
    Ok(TokenStream::from_tokens(tokens))
}
