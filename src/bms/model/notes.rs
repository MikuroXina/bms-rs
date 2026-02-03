//! Note objects management and indexing.
//!
//! This module provides the [`Notes`] struct for managing and querying
//! note objects (WAV objects) in a BMS score, organized by time, channel,
//! and WAV ID for efficient lookup and iteration.

use std::{
    collections::{BTreeMap, HashMap},
    ops::Bound,
};

use itertools::Itertools;

use crate::bms::prelude::*;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct WavObjArena(Vec<WavObj>);

/// Index into the WAV object arena.
///
/// This type represents a stable index that can be used to reference
/// WAV objects stored in the arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WavObjArenaIndex(usize);

impl WavObjArenaIndex {
    /// Returns the contained arena index value.
    #[must_use]
    pub const fn value(self) -> usize {
        self.0
    }
}

/// The playable objects set for querying by lane or time.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Notes {
    /// Arena of `WavObj`, contains the master data of sound objects. `#XXXYY:ZZ...` (note placement)
    arena: WavObjArena,
    /// Note objects index for each wav sound of [`ObjId`].
    idx_by_wav_id: HashMap<ObjId, Vec<WavObjArenaIndex>>,
    /// Note objects index for each channel from the mapping `T`.
    idx_by_channel: HashMap<NoteChannelId, Vec<WavObjArenaIndex>>,
    /// Note objects index sorted by its time.
    idx_by_time: BTreeMap<ObjTime, Vec<WavObjArenaIndex>>,
}

// query methods
impl Notes {
    /// Checks whether there is no valid notes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.all_notes().all(|obj| obj.wav_id.is_null())
    }

    /// Converts into the notes.
    #[must_use]
    pub fn into_all_notes(self) -> Vec<WavObj> {
        self.arena.0
    }

    /// Returns the iterator having all of the notes sorted by time.
    ///
    /// # Note
    /// This iterator may include dangling objects (objects with null `wav_id`) that reference
    /// non-existent WAV files. These dangling objects represent invalid or unassigned notes
    /// and do not affect musical playback.
    /// They may originate from parsing issues in the original BMS file or from user modifications
    /// to the Notes object.
    ///
    /// To filter out dangling objects, use:
    /// ```rust
    /// # let notes = bms_rs::bms::model::Notes::default();
    /// notes.all_notes().filter(|obj| !obj.wav_id.is_null())
    /// # ;
    /// ```
    pub fn all_notes(&self) -> impl Iterator<Item = &WavObj> {
        self.arena.0.iter().sorted()
    }

    /// Returns the iterator having all of the notes and its index sorted by time.
    ///
    /// # Note
    /// This iterator may include dangling objects (objects with null `wav_id`) that reference
    /// non-existent WAV files. These dangling objects represent invalid or unassigned notes
    /// and do not affect musical playback.
    /// They may originate from parsing issues in the original BMS file or from user modifications
    /// to the Notes object.
    ///
    /// To filter out dangling objects, use:
    /// ```rust
    /// # let notes = bms_rs::bms::model::Notes::default();
    /// notes.all_entries().filter(|(_, obj)| !obj.wav_id.is_null())
    /// # ;
    /// ```
    pub fn all_entries(&self) -> impl Iterator<Item = (WavObjArenaIndex, &WavObj)> {
        self.arena
            .0
            .iter()
            .enumerate()
            .sorted_by_key(|obj| obj.1)
            .map(|(idx, obj)| (WavObjArenaIndex(idx), obj))
    }

    /// Returns the iterator having all of the notes in the original insertion order.
    ///
    /// This reflects the order notes were pushed into the arena during parsing, which
    /// corresponds to the lexical order of `Token::Message` entries in the source.
    ///
    /// # Note
    /// This iterator may include dangling objects (objects with null `wav_id`) that reference
    /// non-existent WAV files. These dangling objects represent invalid or unassigned notes
    /// and do not affect musical playback.
    /// They may originate from parsing issues in the original BMS file or from user modifications
    /// to the Notes object.
    ///
    /// **Important**: The insertion order is preserved only until you modify the Notes object
    /// using methods like `retain_notes`, `remove_note`, `pop_note`, etc. After such modifications,
    /// the order may be disrupted as some objects may be replaced with dangling objects.
    ///
    /// To filter out dangling objects, use:
    /// ```rust
    /// # let notes = bms_rs::bms::model::Notes::default();
    /// notes.all_notes_insertion_order().filter(|obj| !obj.wav_id.is_null())
    /// # ;
    /// ```
    pub fn all_notes_insertion_order(&self) -> impl Iterator<Item = &WavObj> {
        self.arena.0.iter()
    }

    /// Returns all the playable notes in the score.
    ///
    /// # Note
    /// This iterator may include dangling objects (objects with null `wav_id`) that reference
    /// non-existent WAV files. These dangling objects represent invalid or unassigned notes
    /// and do not affect musical playback.
    /// They may originate from parsing issues in the original BMS file or from user modifications
    /// to the Notes object.
    ///
    /// To filter out dangling objects, use:
    /// ```rust
    /// # use bms_rs::bms::prelude::*;
    /// # let notes = Notes::default();
    /// notes.playables::<KeyLayoutBeat>().filter(|obj| !obj.wav_id.is_null())
    /// # ;
    /// ```
    pub fn playables<T>(&self) -> impl Iterator<Item = &WavObj>
    where
        T: KeyLayoutMapper,
    {
        self.arena.0.iter().sorted().filter(|obj| {
            obj.channel_id
                .try_into_map::<T>()
                .is_some_and(|map| map.kind().is_playable())
        })
    }

    /// Returns all the displayable notes in the score.
    ///
    /// # Note
    /// This iterator may include dangling objects (objects with null `wav_id`) that reference
    /// non-existent WAV files. These dangling objects represent invalid or unassigned notes
    /// and do not affect musical playback.
    /// They may originate from parsing issues in the original BMS file or from user modifications
    /// to the Notes object.
    ///
    /// To filter out dangling objects, use:
    /// ```rust
    /// # use bms_rs::bms::prelude::*;
    /// # let notes = Notes::default();
    /// notes.displayables::<KeyLayoutBeat>().filter(|obj| !obj.wav_id.is_null())
    /// # ;
    /// ```
    pub fn displayables<T>(&self) -> impl Iterator<Item = &WavObj>
    where
        T: KeyLayoutMapper,
    {
        self.arena.0.iter().sorted().filter(|obj| {
            obj.channel_id
                .try_into_map::<T>()
                .is_some_and(|map| map.kind().is_displayable())
        })
    }

    /// Returns all the bgms in the score.
    ///
    /// # Note
    /// This iterator may include dangling objects (objects with null `wav_id`) that reference
    /// non-existent WAV files. These dangling objects represent invalid or unassigned notes
    /// and do not affect musical playback.
    /// They may originate from parsing issues in the original BMS file or from user modifications
    /// to the Notes object.
    ///
    /// To filter out dangling objects, use:
    /// ```rust
    /// # use bms_rs::bms::prelude::*;
    /// # let notes = Notes::default();
    /// notes.bgms::<KeyLayoutBeat>().filter(|obj| !obj.wav_id.is_null())
    /// # ;
    /// ```
    pub fn bgms<T>(&self) -> impl Iterator<Item = &WavObj>
    where
        T: KeyLayoutMapper,
    {
        self.arena.0.iter().sorted().filter(|obj| {
            obj.channel_id
                .try_into_map::<T>()
                .is_none_or(|map| !map.kind().is_displayable())
        })
    }

    /// Retrieves notes on the specified channel id by the key mapping `T`.
    ///
    /// # Note
    /// This iterator may include dangling objects (objects with null `wav_id`) that reference
    /// non-existent WAV files. These dangling objects represent invalid or unassigned notes
    /// and do not affect musical playback.
    /// They may originate from parsing issues in the original BMS file or from user modifications
    /// to the Notes object.
    ///
    /// To filter out dangling objects, use:
    /// ```rust
    /// # use bms_rs::bms::prelude::*;
    /// # let notes = Notes::default();
    /// let channel_id = NoteChannelId::try_from(['0', '1']).unwrap();
    /// notes.notes_on::<KeyLayoutBeat>(channel_id).filter(|(_, obj)| !obj.wav_id.is_null());
    /// ```
    pub fn notes_on<T>(
        &self,
        channel_id: NoteChannelId,
    ) -> impl Iterator<Item = (WavObjArenaIndex, &WavObj)>
    where
        T: KeyLayoutMapper,
    {
        self.idx_by_channel
            .get(&channel_id)
            .into_iter()
            .flatten()
            .filter_map(|&arena_index| {
                self.arena
                    .0
                    .get(arena_index.0)
                    .map(|obj| (arena_index, obj))
            })
    }

    /// Retrieves notes in the specified time span.
    ///
    /// # Note
    /// This iterator may include dangling objects (objects with null `wav_id`) that reference
    /// non-existent WAV files. These dangling objects represent invalid or unassigned notes
    /// and do not affect musical playback.
    /// They may originate from parsing issues in the original BMS file or from user modifications
    /// to the Notes object.
    ///
    /// To filter out dangling objects, use:
    /// ```rust
    /// # use bms_rs::bms::prelude::*;
    /// # let notes = Notes::default();
    /// let time_span = ObjTime::new(1, 0, 4).unwrap()..ObjTime::new(2, 0, 4).unwrap();
    /// notes.notes_in(time_span).filter(|(_, obj)| !obj.wav_id.is_null());
    /// # ;
    /// ```
    pub fn notes_in<R: std::ops::RangeBounds<ObjTime>>(
        &self,
        time_span: R,
    ) -> impl DoubleEndedIterator<Item = (WavObjArenaIndex, &WavObj)> {
        self.idx_by_time
            .range(time_span)
            .flat_map(|(_, indexes)| indexes)
            .filter_map(|&arena_index| {
                self.arena
                    .0
                    .get(arena_index.0)
                    .map(|obj| (arena_index, obj))
            })
    }

    /// Finds next object on the key `Key` from the time `ObjTime`.
    #[must_use]
    pub fn next_obj_by_key(&self, channel_id: NoteChannelId, time: ObjTime) -> Option<&WavObj> {
        self.notes_in((Bound::Excluded(time), Bound::Unbounded))
            .map(|(_, obj)| obj)
            .find(|obj| obj.channel_id == channel_id)
    }

    /// Gets the latest starting time of all notes.
    #[must_use]
    pub fn last_obj_time(&self) -> Option<ObjTime> {
        let (&time, _) = self.idx_by_time.last_key_value()?;
        Some(time)
    }

    /// Gets the time of last playable object.
    #[must_use]
    pub fn last_playable_time<T>(&self) -> Option<ObjTime>
    where
        T: KeyLayoutMapper,
    {
        self.notes_in(..)
            .map(|(_, obj)| obj)
            .rev()
            .find(|obj| {
                obj.channel_id
                    .try_into_map::<T>()
                    .is_some_and(|map| map.kind().is_displayable())
            })
            .map(|obj| obj.offset)
    }

    /// Gets the time of last BGM object.
    ///
    /// You can't use this to find the length of music. Because this doesn't consider that the length of sound. And visible notes may ring after all BGMs.
    #[must_use]
    pub fn last_bgm_time<T>(&self) -> Option<ObjTime>
    where
        T: KeyLayoutMapper,
    {
        self.notes_in(..)
            .map(|(_, obj)| obj)
            .rev()
            .find(|obj| {
                obj.channel_id
                    .try_into_map::<T>()
                    .is_none_or(|map| !map.kind().is_displayable())
            })
            .map(|obj| obj.offset)
    }
}

// push and remove methods
impl Notes {
    /// Adds the new note object to the notes.
    pub fn push_note(&mut self, note: WavObj) {
        let new_index = WavObjArenaIndex(self.arena.0.len());
        self.idx_by_wav_id
            .entry(note.wav_id)
            .or_default()
            .push(new_index);
        self.idx_by_channel
            .entry(note.channel_id)
            .or_default()
            .push(new_index);
        self.idx_by_time
            .entry(note.offset)
            .or_default()
            .push(new_index);
        self.arena.0.push(note);
    }

    fn remove_index(&mut self, idx: usize, removing: &WavObj) {
        let channel_id = removing.channel_id;
        if let Some(indexes) = self.idx_by_channel.get_mut(&channel_id)
            && let Some(pos) = indexes.iter().position(|id| id.0 == idx)
        {
            indexes.swap_remove(pos);
        }
        if let Some(indexes) = self.idx_by_time.get_mut(&removing.offset)
            && let Some(pos) = indexes.iter().position(|id| id.0 == idx)
        {
            indexes.swap_remove(pos);
        }
    }

    /// Removes the latest note from the notes.
    pub fn pop_note(&mut self) -> Option<WavObj> {
        let last_idx = self.arena.0.len().checked_sub(1)?;
        let last = self.arena.0.pop()?;
        if let Some(indexes) = self.idx_by_wav_id.get_mut(&last.wav_id)
            && let Some(pos) = indexes.iter().position(|id| id.0 == last_idx)
        {
            indexes.swap_remove(pos);
        }
        let channel_id = last.channel_id;
        if let Some(indexes) = self.idx_by_channel.get_mut(&channel_id)
            && let Some(pos) = indexes.iter().position(|id| id.0 == last_idx)
        {
            indexes.swap_remove(pos);
        }
        if let Some(indexes) = self.idx_by_time.get_mut(&last.offset)
            && let Some(pos) = indexes.iter().position(|id| id.0 == last_idx)
        {
            indexes.swap_remove(pos);
        }
        Some(last)
    }

    /// Removes notes belonging to the wav id.
    pub fn remove_note<T>(&mut self, wav_id: ObjId) -> Vec<WavObj>
    where
        T: KeyLayoutMapper,
    {
        let Some(indexes) = self.idx_by_wav_id.remove(&wav_id) else {
            return vec![];
        };
        let mut objs = Vec::with_capacity(indexes.len());
        for WavObjArenaIndex(idx) in indexes {
            let Some(removing_slot) = self.arena.0.get_mut(idx) else {
                continue;
            };
            let removing = std::mem::replace(removing_slot, WavObj::dangling());
            self.remove_index(idx, &removing);
            objs.push(removing);
        }
        objs
    }

    /// Removes a note of the specified index `idx`.
    pub fn pop_by_idx(&mut self, idx: WavObjArenaIndex) -> Option<WavObj> {
        let removing = std::mem::replace(self.arena.0.get_mut(idx.0)?, WavObj::dangling());
        self.remove_index(idx.0, &removing);
        Some(removing)
    }

    /// Removes the latest note using the wav of `wav_id`.
    pub fn pop_latest_of<T>(&mut self, wav_id: ObjId) -> Option<WavObj>
    where
        T: KeyLayoutMapper,
    {
        let &WavObjArenaIndex(to_pop) = self.idx_by_wav_id.get(&wav_id)?.last()?;
        let removing = std::mem::replace(self.arena.0.get_mut(to_pop)?, WavObj::dangling());
        self.remove_index(to_pop, &removing);
        Some(removing)
    }

    /// Adds the BGM (auto-played) note of `wav_id` at `time`.
    pub fn push_bgm<T>(&mut self, time: ObjTime, wav_id: ObjId)
    where
        T: KeyLayoutMapper,
    {
        self.push_note(WavObj {
            offset: time,
            channel_id: NoteChannelId::bgm(),
            wav_id,
        });
    }

    /// Retains note objects with the condition `cond`. It keeps only the [`WavObj`]s which `cond` returned `true`.
    pub fn retain_notes<T, F: FnMut(&WavObj) -> bool>(&mut self, mut cond: F)
    where
        T: KeyLayoutMapper,
    {
        let removing_indexes: Vec<_> = self
            .arena
            .0
            .iter()
            .enumerate()
            .filter(|&(_, obj)| !cond(obj))
            .map(|(i, _)| i)
            .collect();
        for removing_idx in removing_indexes {
            let Some(removing_slot) = self.arena.0.get_mut(removing_idx) else {
                continue;
            };
            let removing = std::mem::replace(removing_slot, WavObj::dangling());
            self.remove_index(removing_idx, &removing);
        }
    }

    /// Duplicates the object with id `src` at the time `at` into the channel of id `dst`.
    pub fn dup_note_into(&mut self, src: ObjId, at: ObjTime, dst: NoteChannelId) {
        let Some(src_obj) = self
            .idx_by_wav_id
            .get(&src)
            .into_iter()
            .flatten()
            .filter_map(|idx| self.arena.0.get(idx.0))
            .find(|obj| obj.offset == at)
        else {
            return;
        };
        let new = WavObj {
            channel_id: dst,
            ..*src_obj
        };
        self.push_note(new);
    }
}

// modify methods
impl Notes {
    /// Changes the channel of notes `target` in `time_span` into another channel `dst`.
    pub fn change_note_channel<I>(&mut self, targets: I, dst: NoteChannelId)
    where
        I: IntoIterator<Item = WavObjArenaIndex>,
    {
        for target in targets {
            let Some(obj) = self.arena.0.get_mut(target.0) else {
                continue;
            };

            // Drain all ids from ids_by_channel where channel id matches
            let src = obj.channel_id;
            if let Some(indexes) = self.idx_by_channel.get_mut(&src)
                && let Some(pos) = indexes.iter().position(|&idx| idx == target)
            {
                indexes.swap_remove(pos);
            }
            self.idx_by_channel.entry(dst).or_default().push(target);

            // Modify entry
            obj.channel_id = dst;
        }
    }

    /// Changes the specified object `target`'s offset time into `new_time`.
    pub fn change_note_time(
        &mut self,
        target: WavObjArenaIndex,
        new_time: ObjTime,
    ) -> Option<ObjTime> {
        let to_change = self.arena.0.get_mut(target.0)?;
        let old_time = to_change.offset;
        if old_time == new_time {
            return Some(new_time);
        }

        if let Some(indexes) = self.idx_by_time.get_mut(&old_time) {
            let idx_by_time = indexes.iter().position(|&idx| idx == target)?;
            indexes.swap_remove(idx_by_time);
        }
        self.idx_by_time.entry(new_time).or_default().push(target);
        to_change.offset = new_time;
        Some(old_time)
    }
}

#[cfg(test)]
mod tests {

    use super::Notes;
    use crate::bms::prelude::*;

    #[test]
    fn push_and_pop() {
        let mut notes = Notes::default();
        let note = WavObj {
            offset: ObjTime::new(1, 2, 4).expect("4 should be a valid denominator"),
            channel_id: NoteChannelId::bgm(),
            wav_id: ObjId::try_from("01", false).unwrap(),
        };

        assert!(notes.pop_note().is_none());

        notes.push_note(note.clone());
        let removed = notes.pop_note();
        assert_eq!(Some(note), removed);

        assert!(notes.pop_note().is_none());
    }

    #[test]
    fn change_note_channel() {
        let mut notes = Notes::default();
        let note = WavObj {
            offset: ObjTime::new(1, 2, 4).expect("4 should be a valid denominator"),
            channel_id: NoteChannelId::bgm(),
            wav_id: ObjId::try_from("01", false).unwrap(),
        };

        assert!(notes.pop_note().is_none());

        notes.push_note(note);
        let (idx, _) = notes.all_entries().next().unwrap();
        notes.change_note_channel(
            [idx],
            KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1)).to_channel_id(),
        );

        assert_eq!(
            notes.all_notes().next(),
            Some(&WavObj {
                offset: ObjTime::new(1, 2, 4,).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: ObjId::try_from("01", false).unwrap(),
            })
        );
    }

    #[test]
    fn change_note_time() {
        let mut notes = Notes::default();
        let note = WavObj {
            offset: ObjTime::new(1, 2, 4).expect("4 should be a valid denominator"),
            channel_id: NoteChannelId::bgm(),
            wav_id: ObjId::try_from("01", false).unwrap(),
        };

        assert!(notes.pop_note().is_none());

        notes.push_note(note);
        let (idx, _) = notes.all_entries().next().unwrap();
        notes.change_note_time(
            idx,
            ObjTime::new(1, 1, 4).expect("4 should be a valid denominator"),
        );

        assert_eq!(
            notes.all_notes().next(),
            Some(&WavObj {
                offset: ObjTime::new(1, 1, 4,).expect("4 should be a valid denominator"),
                channel_id: NoteChannelId::bgm(),
                wav_id: ObjId::try_from("01", false).unwrap(),
            })
        );
    }
}
