mod header;

use rand::Rng;

use self::header::Header;
use crate::lex::token::{Token, TokenStream};

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
    pub fn from_token_stream(token_stream: &TokenStream, mut rng: impl Rng) -> Result<Self> {
        let mut random_stack = vec![];
        let mut in_ignored_clause = false;
        let mut header = Header::default();

        for token in token_stream.iter() {
            match *token {
                Token::Random(rand_max) => random_stack.push(rng.gen_range(1..=rand_max)),
                Token::EndRandom => {
                    random_stack.pop();
                }
                Token::If(rand_target) => {
                    in_ignored_clause = Some(&rand_target) != random_stack.last()
                }
                Token::ElseIf(rand_target) => {
                    in_ignored_clause = Some(&rand_target) != random_stack.last()
                }
                Token::EndIf => in_ignored_clause = false,
                _ => {}
            }
            if in_ignored_clause {
                continue;
            }
            header.parse(token)?;
        }

        Ok(Self { header })
    }
}
