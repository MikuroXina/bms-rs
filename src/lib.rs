use self::token::TokenStream;

pub mod command;
pub mod token;

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ParseError {}

pub fn parse(source: &str) -> Result<TokenStream, ParseError> {
    todo!()
}
