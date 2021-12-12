//! Parser for BMS format. The reason why the implementation separated into lex and parse is the score may contain some randomized elements such as `#RANDOM`. This separation make us able to parse the tokens with the custom random generator cheaply.

mod header;
mod random;
pub mod rng;

use itertools::Itertools;
use std::{
    collections::{BTreeMap, HashMap},
    ops::ControlFlow,
};

use self::{header::Header, random::RandomParser, rng::Rng};
use crate::lex::{
    command::{Channel, Key, NoteKind, ObjId},
    token::{Token, TokenStream},
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

/// The objects set for querying by lane or time.
#[derive(Debug, Default)]
pub struct Notes {
    objs: HashMap<ObjId, Obj>,
    ids_by_key: HashMap<Key, BTreeMap<ObjTime, ObjId>>,
}

impl Notes {
    /// Returns the iterator having all of the notes sorted by time.
    pub fn into_all_notes(self) -> Vec<Obj> {
        self.objs.into_values().sorted().collect()
    }

    /// Adds the new note object to the notes.
    pub fn push(&mut self, note: Obj) {
        self.objs.insert(note.obj, note.clone());
        self.ids_by_key
            .entry(note.key)
            .or_insert_with(BTreeMap::new)
            .insert(note.offset, note.obj);
    }

    /// Removes the note from the notes.
    pub fn remove(&mut self, id: ObjId) -> Option<Obj> {
        self.objs.remove(&id).map(|removed| {
            self.ids_by_key
                .get_mut(&removed.key)
                .unwrap()
                .remove(&removed.offset)
                .unwrap();
            removed
        })
    }

    pub(crate) fn parse(&mut self, token: &Token) -> Result<()> {
        match token {
            Token::Message {
                track,
                channel:
                    Channel::Note {
                        kind,
                        is_player1,
                        key,
                    },
                message,
            } => {
                let denominator = message.len() as u32 / 2;
                for (i, (c1, c2)) in message.chars().tuples().into_iter().enumerate() {
                    let id = c1.to_digit(36).unwrap() * 36 + c2.to_digit(36).unwrap();
                    if id == 0 {
                        continue;
                    }
                    let obj = (id as u16).try_into().unwrap();
                    self.push(Obj {
                        offset: ObjTime::new(track.0, i as u32, denominator),
                        kind: *kind,
                        is_player1: *is_player1,
                        key: *key,
                        obj,
                    });
                }
            }
            &Token::LnObj(end_id) => {
                let mut end_note = self
                    .remove(end_id)
                    .ok_or(ParseError::UndefinedObject(end_id))?;
                let Obj { offset, key, .. } = &end_note;
                let (_, &begin_id) =
                    self.ids_by_key[key].range(..offset).last().ok_or_else(|| {
                        ParseError::SyntaxError(format!(
                            "expected preceding object for #LNOBJ {:?}",
                            end_id
                        ))
                    })?;
                let mut begin_note = self.remove(begin_id).unwrap();
                begin_note.kind = NoteKind::Long;
                end_note.kind = NoteKind::Long;
                self.push(begin_note);
                self.push(end_note);
            }
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
