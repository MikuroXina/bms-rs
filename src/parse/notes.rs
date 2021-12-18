//! Note objects manager.

use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};

use super::{
    header::Header,
    obj::{Obj, ObjTime},
    ParseError, Result,
};
use crate::lex::{
    command::{Channel, Key, NoteKind, ObjId},
    token::Token,
};

/// An object to change the BPM of the score.
#[derive(Debug, Clone, Copy)]
pub struct BpmChangeObj {
    /// The time to begin the change of BPM.
    pub time: ObjTime,
    /// The BPM to be.
    pub bpm: f64,
}

impl PartialEq for BpmChangeObj {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for BpmChangeObj {}

impl PartialOrd for BpmChangeObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BpmChangeObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// An object to change its section length of the score.
#[derive(Debug, Clone, Copy)]
pub struct SectionLenChangeObj {
    /// The time to begin the change of section length.
    pub time: ObjTime,
    /// The length to be.
    pub length: f64,
}

impl PartialEq for SectionLenChangeObj {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for SectionLenChangeObj {}

impl PartialOrd for SectionLenChangeObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SectionLenChangeObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// The objects set for querying by lane or time.
#[derive(Debug, Default)]
pub struct Notes {
    objs: HashMap<ObjId, Obj>,
    ids_by_key: HashMap<Key, BTreeMap<ObjTime, ObjId>>,
    bpm_changes: BTreeMap<ObjTime, BpmChangeObj>,
    section_len_changes: BTreeMap<ObjTime, SectionLenChangeObj>,
}

impl Notes {
    /// Converts into the notes sorted by time.
    pub fn into_all_notes(self) -> Vec<Obj> {
        self.objs.into_values().sorted().collect()
    }

    /// Returns the iterator having all of the notes sorted by time.
    pub fn all_notes(&self) -> impl Iterator<Item = &Obj> {
        self.objs.values().sorted()
    }

    /// Returns the bpm change objects.
    pub fn bpm_changes(&self) -> &BTreeMap<ObjTime, BpmChangeObj> {
        &self.bpm_changes
    }

    /// Returns the section len change objects.
    pub fn section_len_changes(&self) -> &BTreeMap<ObjTime, SectionLenChangeObj> {
        &self.section_len_changes
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

    pub(crate) fn parse(&mut self, token: &Token, header: &Header) -> Result<()> {
        match token {
            Token::Message {
                track,
                channel: Channel::BpmChange,
                message,
            } => {
                let denominator = message.len() as u32 / 2;
                for (i, (c1, c2)) in message.chars().tuples().into_iter().enumerate() {
                    let id = c1.to_digit(36).unwrap() * 36 + c2.to_digit(36).unwrap();
                    if id == 0 {
                        continue;
                    }
                    let obj = (id as u16).try_into().unwrap();
                    let time = ObjTime::new(track.0, i as u32, denominator);
                    let &bpm = header
                        .bpm_changes
                        .get(&obj)
                        .ok_or(ParseError::UndefinedObject(obj))?;
                    if self
                        .bpm_changes
                        .insert(time, BpmChangeObj { time, bpm })
                        .is_some()
                    {
                        eprintln!("duplicate bpm change object detected at {:?}", time);
                    }
                }
            }
            Token::Message {
                track,
                channel: Channel::BpmChangeU8,
                message,
            } => {
                let denominator = message.len() as u32 / 2;
                for (i, (c1, c2)) in message.chars().tuples().into_iter().enumerate() {
                    let bpm = c1.to_digit(16).unwrap() * 16 + c2.to_digit(16).unwrap();
                    if bpm == 0 {
                        continue;
                    }
                    let time = ObjTime::new(track.0, i as u32, denominator);
                    if self
                        .bpm_changes
                        .insert(
                            time,
                            BpmChangeObj {
                                time,
                                bpm: bpm as f64,
                            },
                        )
                        .is_some()
                    {
                        eprintln!("duplicate bpm change object detected at {:?}", time);
                    }
                }
            }
            Token::Message {
                track,
                channel: Channel::SectionLen,
                message,
            } => {
                let time = ObjTime::new(track.0, 0, 4);
                let length = message.parse().expect("f64 as section length");
                assert!(0.0 < length, "section length must be greater than zero");
                if self
                    .section_len_changes
                    .insert(time, SectionLenChangeObj { time, length })
                    .is_some()
                {
                    eprintln!("duplicate bpm change object detected at {:?}", time);
                }
            }
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
