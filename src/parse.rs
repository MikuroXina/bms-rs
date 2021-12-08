mod header;

use std::{collections::BinaryHeap, ops::RangeInclusive};

use self::header::Header;
use crate::lex::{
    command::{Channel, ObjId},
    token::{Token, TokenStream},
};

#[derive(Debug, Clone)]
pub enum ParseError {
    SyntaxError(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::SyntaxError(mes) => write!(f, "syntax error: {}", mes),
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

pub trait Rng {
    fn gen(&mut self, range: RangeInclusive<u32>) -> u32;
}

#[derive(Debug)]
pub struct Bms {
    pub header: Header,
    pub sorted_notes: Vec<Obj>,
}

impl Bms {
    pub fn from_token_stream(token_stream: &TokenStream, mut rng: impl Rng) -> Result<Self> {
        enum ClauseState {
            Random(u32),
            If(bool),
        }
        let mut random_stack = vec![];
        let mut notes_heap = BinaryHeap::with_capacity(
            token_stream
                .iter()
                .filter(|token| matches!(token, Token::Message { .. }))
                .count(),
        );
        let mut header = Header::default();

        for token in token_stream.iter() {
            match *token {
                Token::If(rand_target) => {
                    if let Some(&ClauseState::Random(rand)) = random_stack.last() {
                        random_stack.push(ClauseState::If(rand_target == rand));
                        continue;
                    }
                    return Err(ParseError::SyntaxError(
                        "#IF command must be in #RANDOM - #ENDRANDOM block".into(),
                    ));
                }
                Token::ElseIf(rand_target) => {
                    if let Some(ClauseState::If(_)) = random_stack.last() {
                        random_stack.pop();
                        let rand = match random_stack.last().unwrap() {
                            &ClauseState::Random(rand) => rand,
                            ClauseState::If(_) => unreachable!(),
                        };
                        random_stack.push(ClauseState::If(rand_target == rand));
                        continue;
                    }
                    return Err(ParseError::SyntaxError(
                        "#ELSEIF command must come after #IF block".into(),
                    ));
                }
                Token::EndIf => {
                    if let Some(ClauseState::If(_)) = random_stack.last() {
                        random_stack.pop();
                        continue;
                    }
                    return Err(ParseError::SyntaxError(
                        "#ENDIF command must come after #IF or #ELSEIF block".into(),
                    ));
                }
                Token::Random(rand_max) => {
                    if let Some(&ClauseState::Random(_)) = random_stack.last() {
                        return Err(ParseError::SyntaxError(
                            "#RANDOM command must come in root or #IF block".into(),
                        ));
                    }
                    if let Some(ClauseState::If(false)) = random_stack.last() {
                        random_stack.push(ClauseState::Random(0));
                        continue;
                    }
                    random_stack.push(ClauseState::Random(rng.gen(1..=rand_max)));
                    continue;
                }
                Token::EndRandom => {
                    if let Some(&ClauseState::Random(_)) = random_stack.last() {
                        random_stack.pop();
                        continue;
                    }
                    return Err(ParseError::SyntaxError(
                        "#ENDRANDOM command must come after #RANDOM block".into(),
                    ));
                }
                _ => {}
            }
            if let Some(ClauseState::Random(_) | ClauseState::If(false)) = random_stack.last() {
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
