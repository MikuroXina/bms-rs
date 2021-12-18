//! Note objects manager.

use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};

use super::{
    obj::{Obj, ObjTime},
    ParseError, Result,
};
use crate::lex::{
    command::{Channel, Key, NoteKind, ObjId},
    token::Token,
};

/// The objects set for querying by lane or time.
#[derive(Debug, Default)]
pub struct Notes {
    objs: HashMap<ObjId, Obj>,
    ids_by_key: HashMap<Key, BTreeMap<ObjTime, ObjId>>,
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
