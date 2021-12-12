//! Parser for BMS format. The reason why the implementation separated into lex and parse is the score may contain some randomized elements such as `#RANDOM`. This separation make us able to parse the tokens with the custom random generator cheaply.

mod header;
mod random;
pub mod rng;

use itertools::Itertools;
use std::{collections::BinaryHeap, ops::ControlFlow};

use self::{header::Header, random::RandomParser, rng::Rng};
use crate::lex::{
    command::{Channel, ObjId},
    token::{Token, TokenStream},
};

/// An error occurred when parsing the [`TokenStream`].
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Syntax formed from the commands was invalid.
    SyntaxError(String),
    /// The invalid real number for the BPM.
    BpmParseError(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::SyntaxError(mes) => write!(f, "syntax error: {}", mes),
            ParseError::BpmParseError(bpm) => write!(f, "not a number bpm: {}", bpm),
        }
    }
}

impl std::error::Error for ParseError {}

/// A custom result type for parsing.
pub type Result<T> = std::result::Result<T, ParseError>;

/// An object on the score.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Obj {
    /// The track, or measure, where the object is in.
    pub track: u32,
    /// The time offset numerator in the track.
    pub time_numerator_in_track: u32,
    /// The time offset denominator in the track.
    pub time_denominator_in_track: u32,
    /// The channel, or lane, where the object is placed.
    pub channel: Channel,
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
        self.track.cmp(&other.track).then_with(|| {
            let self_time_in_track = self.time_numerator_in_track * other.time_denominator_in_track;
            let other_time_in_track =
                other.time_numerator_in_track * self.time_denominator_in_track;
            self_time_in_track.cmp(&other_time_in_track)
        })
    }
}

/// A score data of BMS format.
#[derive(Debug)]
pub struct Bms {
    /// The header data in the score.
    pub header: Header,
    /// The sound objects sorted by its time.
    pub sorted_notes: Vec<Obj>,
}

impl Bms {
    /// Parses a token stream into [`Bms`] with a random generator [`Rng`].
    pub fn from_token_stream(token_stream: &TokenStream, rng: impl Rng) -> Result<Self> {
        let mut random_parser = RandomParser::new(rng);
        let mut notes_heap = BinaryHeap::with_capacity(
            token_stream
                .iter()
                .filter(|token| matches!(token, Token::Message { .. }))
                .count(),
        );
        let mut header = Header::default();

        for token in token_stream.iter() {
            match random_parser.parse(token) {
                ControlFlow::Continue(_) => {}
                ControlFlow::Break(Ok(_)) => continue,
                ControlFlow::Break(Err(e)) => return Err(e),
            }
            match token {
                Token::Message {
                    track,
                    channel,
                    message,
                } if channel != &Channel::SectionLen => {
                    let time_denominator_in_track = message.len() as u32 / 2;
                    for (i, (c1, c2)) in message.chars().tuples().into_iter().enumerate() {
                        let mut id = String::new();
                        id.push(c1);
                        id.push(c2);
                        let id: u16 = id.parse().unwrap();
                        if id == 0 {
                            continue;
                        }
                        let obj = id.try_into().unwrap();
                        notes_heap.push(Obj {
                            track: track.0,
                            time_numerator_in_track: i as u32 + 1,
                            time_denominator_in_track,
                            channel: channel.clone(),
                            obj,
                        });
                    }
                    continue;
                }
                Token::LnObj(end_id) => {}
                _ => {}
            }
            header.parse(token)?;
        }

        Ok(Self {
            header,
            sorted_notes: notes_heap.into_sorted_vec(),
        })
    }
}
