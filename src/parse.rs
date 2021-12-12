//! Parser for BMS format. The reason why the implementation separated into lex and parse is the score may contain some randomized elements such as `#RANDOM`. This separation make us able to parse the tokens with the custom random generator cheaply.

mod header;
pub mod notes;
mod random;
pub mod rng;

use std::ops::ControlFlow;

use self::{header::Header, notes::Notes, random::RandomParser, rng::Rng};
use crate::lex::{
    command::{Key, NoteKind, ObjId},
    token::TokenStream,
};

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

/// A time of the object on the score.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjTime {
    /// The track, or measure, where the object is in.
    pub track: u32,
    /// The time offset numerator in the track.
    pub numerator: u32,
    /// The time offset denominator in the track.
    pub denominator: u32,
}

impl ObjTime {
    /// Create a new time.
    ///
    /// # Panics
    ///
    /// Panics if `denominator` is 0 or `numerator` is greater than or equal to `denominator`.
    pub fn new(track: u32, numerator: u32, denominator: u32) -> Self {
        if track == 0 {
            eprintln!("warning: track 000 detected");
        }
        assert!(0 < denominator);
        assert!(numerator < denominator);
        Self {
            track,
            numerator,
            denominator,
        }
    }
}

impl PartialOrd for ObjTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ObjTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_time_in_track = self.numerator * other.denominator;
        let other_time_in_track = other.numerator * self.denominator;
        self.track
            .cmp(&other.track)
            .then(self_time_in_track.cmp(&other_time_in_track))
    }
}

/// An object on the score.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Obj {
    /// The time offset in the track.
    pub offset: ObjTime,
    /// THe note kind of the the object.
    pub kind: NoteKind,
    /// Whether the note is for player 1.
    pub is_player1: bool,
    /// The key, or lane, where the object is placed.
    pub key: Key,
    /// The id of the object.
    pub obj: ObjId,
}

impl PartialOrd for Obj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Obj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.offset
            .cmp(&other.offset)
            .then(self.obj.cmp(&other.obj))
    }
}

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
            notes.parse(token)?;
            header.parse(token)?;
        }

        Ok(Self { header, notes })
    }
}
