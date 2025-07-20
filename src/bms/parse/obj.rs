//! Definitions of the note object.

use crate::{
    lex::command::{Key, NoteKind, ObjId, PlayerSide},
    time::ObjTime,
};

/// An object on the score.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Obj {
    /// The time offset in the track.
    pub offset: ObjTime,
    /// THe note kind of the the object.
    pub kind: NoteKind,
    /// The side of the player.
    pub side: PlayerSide,
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
