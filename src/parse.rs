mod header;

use rand::Rng;
use std::collections::BinaryHeap;

use self::header::Header;
use crate::lex::{
    command::{Channel, ObjId},
    token::{Token, TokenStream},
};

#[derive(Debug, Clone)]
pub enum ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for ParseError {}

pub type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Note {
    pub track: u32,
    pub time_numerator_in_track: u32,
    pub time_denominator_in_track: u32,
    pub channel: Channel,
    pub obj: ObjId,
}

impl PartialOrd for Note {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Note {
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
    pub sorted_notes: Vec<Note>,
}

impl Bms {
    pub fn from_token_stream(token_stream: &TokenStream, mut rng: impl Rng) -> Result<Self> {
        let mut random_stack = vec![];
        let mut in_ignored_clause = false;
        let mut notes_heap = BinaryHeap::with_capacity(
            token_stream
                .iter()
                .filter(|token| matches!(token, Token::Message { .. }))
                .count(),
        );
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
                    notes_heap.push(Note {
                        track: track.0,
                        time_numerator_in_track: i as u32,
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
