mod header;
mod random;
pub mod rng;

use std::{collections::BinaryHeap, ops::ControlFlow};

use self::{header::Header, random::RandomParser, rng::Rng};
use crate::lex::{
    command::{Channel, ObjId},
    token::{Token, TokenStream},
};

#[derive(Debug, Clone)]
pub enum ParseError {
    SyntaxError(String),
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

pub type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Obj {
    pub track: u32,
    pub time_numerator_in_track: u32,
    pub time_denominator_in_track: u32,
    pub channel: Channel,
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

#[derive(Debug)]
pub struct Bms {
    pub header: Header,
    pub sorted_notes: Vec<Obj>,
}

impl Bms {
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
            if let Token::Message {
                track,
                channel,
                message,
            } = token
            {
                for (i, obj) in message
                    .iter()
                    .enumerate()
                    .map(|(i, obj)| obj.map(|obj| (i, obj)))
                    .flatten()
                {
                    notes_heap.push(Obj {
                        track: track.0,
                        time_numerator_in_track: i as u32 + 1,
                        time_denominator_in_track: message.len() as u32,
                        channel: channel.clone(),
                        obj,
                    });
                }
                continue;
            }
            header.parse(token)?;
        }

        Ok(Self {
            header,
            sorted_notes: notes_heap.into_sorted_vec(),
        })
    }
}
