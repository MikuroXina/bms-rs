use super::Result;
use crate::lex::token::Token;

#[derive(Debug, Default)]
pub struct Header {}

impl Header {
    pub fn parse(&mut self, token: &Token) -> Result<()> {
        todo!()
    }
}
