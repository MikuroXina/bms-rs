//! Definitions of the note object.

use crate::lex::command::{Key, NoteKind, ObjId};

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
