//! The BMS format parser.
//!
//! Be-Music Source, called BMS for short, is a file format devised by Urao Yane in 1998 for a simulator of the game Beatmania by KONAMI. This describes what and when notes are arranged and its music metadata. It is a plain text file with some "command" lines starting with `#` character.
//!
//! # Usage
//!
//! At first, you can get the tokens stream with [`lex::parse`]. Then pass it and the random generator to [`parse::Bms::from_token_stream`] to get the notes data. Because BMS format has some randomized syntax.
//!
//! ```
//! use bms_rs::{
//!     lex::parse,
//!     parse::{rng::RngMock, Bms},
//! };
//!
//! let source = std::fs::read_to_string("tests/lilith_mx.bms").unwrap();
//! let token_stream = parse(&source).expect("must be parsed");
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
//! Message command starts with `#XXXYY:ZZ...`. `XXX` is the number of the measure, `YY` is the channel of the message, and `ZZ...` is the object id sequence.
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
//! 002|--|03------------|
//!    |  |  []  []  []  |
//!    |()|[]  []  []  []|
//!    |-----------------|
//! ```

#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![cfg_attr(feature = "nightly", feature(doc_cfg))]

#[cfg(feature = "bmson")]
pub mod bmson;
pub mod lex;
pub mod parse;
pub mod time;

use thiserror::Error;

use self::{lex::LexError, parse::ParseError};

/// An error occurred when parsing the BMS format file.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum BmsError {
    /// An error comes from lexical analyzer.
    #[error("lex error: {0}")]
    LexError(LexError),
    /// An error comes from syntax parser.
    #[error("parse error: {0}")]
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

/// A custom result type for bms-rs.
pub type Result<T> = std::result::Result<T, BmsError>;
