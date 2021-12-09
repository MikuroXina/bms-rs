//! The BMS format parser.
//!
//! Be-Music Source, called BMS for short, is a file format devised by Urao Yane in 1998 for a simulator of the game Beatmania by KONAMI. This describes what and when notes are arranged and its music metadata. It is a plain text file with some "command" lines starting with `#` character.
//!
//! # Usage
//!
//! ```
//! use bms_rs::{
//!     lex::parse,
//!     parse::{rng::RngMock, Bms},
//! };
//!
//! let source = std::fs::read_to_string("tests/lilith_mx.bms");
//! let token_stream = parse(source).expect("must be parsed");
//! let rng = RngMock([1]);
//! let bms = Bms::from_token_stream(&token_stream, rng).expect("must be parsed");
//! ```
//!
//! # About the format
//!
//! ## Command
//!
//! Each command starts with `#` character, and other lines will be ignored. Some commands require arguments separated by whitespace character such as spaces or tabs.
//!
//! ```text
//! #PLAYER 1
//! #GENRE FUGA
//! #TITLE BAR(^^)
//! #ARTIST MikuroXina
//! #BPM 120
//! #PLAYLEVEL 6
//! #RANK 2
//!
//! #WAV01 hoge.WAV
//! #WAV02 foo.WAV
//! #WAV03 bar.WAV
//!
//! #00211:0303030303
//! ```
//!
//! ### Header command
//!
//! Header commands are used to express the metadata of the music or the definition for note arrangement.
//!
//! ### Message command
//!
//! Message command starts with `#XXXYY:ZZ.... XXX` is the number of the measure, `YY` is the channel of the message, and `ZZ...` is the object id sequence.
//!
//! The measure must start from 1, but some player may allow the 0 measure (i.e. Lunatic Rave 2).
//!
//! The channel commonly expresses what the lane be arranged the note to.
//!
//! The object id is formed by 2-digit of 36-radix (`[0-9a-zA-Z]`) integer. So the sequence length must be an even number. The `00` object id is the special id, expresses the rest (no object lies). The object lies on the position divided equally by how many the object is in the measure. For example:
//!
//! ```text
//! #00211:0303000303
//! ```
//!
//! This will be placed as:
//!
//! ```text
//! 003|--|--------------|
//!    |  |03            |
//!    |  |03            |
//!    |  |              |
//!    |  |03            |
//!    |  |03            |
//! 002|--|--------------|
//!    |  |  []  []  []  |
//!    |()|[]  []  []  []|
//!    |-----------------|
//! ```

#![warn(missing_docs)]

pub mod lex;
pub mod parse;

use self::{lex::LexError, parse::ParseError};

/// An error occurred when parsing the BMS format file.
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
