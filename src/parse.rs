mod header;

use rand::Rng;

use self::header::Header;
use crate::lex::token::TokenStream;

#[derive(Debug, Clone)]
pub enum ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for ParseError {}

pub type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug)]
pub struct Bms {
    pub header: Header,
}

impl Bms {
    pub fn from_token_stream(token_stream: &TokenStream, rng: impl Rng) -> Result<Self> {
        let mut header = Header::default();

        for token in token_stream.iter() {
            header.parse(token)?;
        }

        Ok(Self { header })
    }
}
