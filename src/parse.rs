//! Parser for BMS format. The reason why the implementation separated into lex and parse is the score may contain some randomized elements such as `#RANDOM`. This separation make us able to parse the tokens with the custom random generator cheaply.

mod header;
mod random;
pub mod rng;

use itertools::Itertools;
use std::{
    collections::{BinaryHeap, HashMap},
    ops::ControlFlow,
};

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

/// A time of the object on the score.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjTime {
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
    pub fn new(numerator: u32, denominator: u32) -> Self {
        assert!(0 < denominator);
        assert!(numerator < denominator);
        Self {
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
        self_time_in_track.cmp(&other_time_in_track)
    }
}

/// An object on the score.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Obj {
    /// The track, or measure, where the object is in.
    pub track: u32,
    /// The time offset in the track.
    pub offset: ObjTime,
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
        self.track
            .cmp(&other.track)
            .then(self.offset.cmp(&other.offset))
            .then(self.obj.cmp(&other.obj))
    }
}

/// The objects set for querying by channel or time.
#[derive(Debug)]
pub struct Notes(HashMap<Channel, BinaryHeap<Obj>>);

impl Notes {
    /// Returns the iterator having all of the notes sorted by time.
    pub fn into_all_notes(self) -> Vec<Obj> {
        self.0
            .into_values()
            .reduce(|mut a, mut b| {
                a.append(&mut b);
                a
            })
            .map(|heap| heap.into_sorted_vec())
            .unwrap_or_default()
    }

    fn push(&mut self, note: Obj) {
        let note = &note;
        self.0
            .entry(note.channel.clone())
            .and_modify(move |heap| {
                heap.push(note.clone());
            })
            .or_insert_with(move || {
                let mut heap = BinaryHeap::new();
                heap.push(note.clone());
                heap
            });
    }

    pub(crate) fn parse(&mut self, token: &Token) -> Result<()> {
        match token {
            Token::Message {
                track,
                channel,
                message,
            } if channel != &Channel::SectionLen => {
                let denominator = message.len() as u32 / 2;
                for (i, (c1, c2)) in message.chars().tuples().into_iter().enumerate() {
                    let id = c1.to_digit(36).unwrap() * 36 + c2.to_digit(36).unwrap();
                    if id == 0 {
                        continue;
                    }
                    let obj = (id as u16).try_into().unwrap();
                    self.push(Obj {
                        track: track.0,
                        offset: ObjTime::new(i as u32, denominator),
                        channel: channel.clone(),
                        obj,
                    });
                }
            }
            Token::LnObj(end_id) => todo!(),
            Token::AtBga {
                id,
                source_bmp,
                trim_top_left,
                trim_size,
                draw_point,
            } => todo!(),
            Token::Bga {
                id,
                source_bmp,
                trim_top_left,
                trim_bottom_right,
                draw_point,
            } => todo!(),
            Token::ChangeOption(_, _) => todo!(),
            Token::ExBmp(_, _, _) => todo!(),
            Token::ExRank(_, _) => todo!(),
            Token::ExWav(_, _, _) => todo!(),
            Token::Text(_, _) => todo!(),
            _ => {}
        }
        Ok(())
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
        let mut notes = Notes(HashMap::new());
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
