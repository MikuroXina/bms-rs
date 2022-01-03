//! Parser for BMS format. The reason why the implementation separated into lex and parse is the score may contain some randomized elements such as `#RANDOM`. This separation make us able to parse the tokens with the custom random generator cheaply.

pub mod header;
pub mod notes;
pub mod obj;
mod random;
pub mod rng;

use std::ops::ControlFlow;

use self::{header::Header, notes::Notes, random::RandomParser, rng::Rng};
use crate::lex::{command::ObjId, token::TokenStream};

/// An error occurred when parsing the [`TokenStream`].
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Syntax formed from the commands was invalid.
    SyntaxError(String),
    /// The invalid real number for the BPM.
    BpmParseError(String),
    /// The object has required but not defined,
    UndefinedObject(ObjId),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::SyntaxError(mes) => write!(f, "syntax error: {}", mes),
            ParseError::BpmParseError(bpm) => write!(f, "not a number bpm: {}", bpm),
            ParseError::UndefinedObject(id) => write!(f, "undefined object: {:?}", id),
        }
    }
}

impl std::error::Error for ParseError {}

/// A custom result type for parsing.
pub type Result<T> = std::result::Result<T, ParseError>;

/// A score data of BMS format.
#[derive(Debug)]
pub struct Bms {
    /// The header data in the score.
    pub header: Header,
    /// The objects in the score.
    pub notes: Notes,
}

impl Bms {
    /// Parses a token stream into [`Bms`] with a random generator [`Rng`].
    pub fn from_token_stream(token_stream: &TokenStream, rng: impl Rng) -> Result<Self> {
        let mut random_parser = RandomParser::new(rng);
        let mut notes = Notes::default();
        let mut header = Header::default();

        for token in token_stream.iter() {
            match random_parser.parse(token) {
                ControlFlow::Continue(_) => {}
                ControlFlow::Break(Ok(_)) => continue,
                ControlFlow::Break(Err(e)) => return Err(e),
            }
            notes.parse(token, &header)?;
            header.parse(token)?;
        }

        Ok(Self { header, notes })
    }
}
