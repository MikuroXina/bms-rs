use std::{
    collections::{BTreeMap, HashMap},
    marker::PhantomData,
    ops::Bound,
    path::PathBuf,
};

use itertools::Itertools;

use crate::{
    bms::prelude::*,
    parse::{Result, prompt::ChannelDuplication},
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct WavObjArena(Vec<WavObj>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WavObjArenaIndex(usize);

/// The playable objects set for querying by lane or time.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Notes<T> {
    /// The path to override the base path of the WAV file path.
    /// This allows WAV files to be referenced relative to a different directory.
    pub wav_path_root: Option<PathBuf>,
    /// The WAV file paths corresponding to the id of the note object.
    pub wav_files: HashMap<ObjId, PathBuf>,
    /// Arena of `WavObj`, contains the master data of sound objects. `#XXXYY:ZZ...` (note placement)
    arena: WavObjArena,
    /// Note objects index for each wav sound of [`ObjId`].
    idx_by_wav_id: HashMap<ObjId, Vec<WavObjArenaIndex>>,
    /// Note objects index for each channel from the mapping `T`.
    idx_by_channel: HashMap<NoteChannelId, Vec<WavObjArenaIndex>>,
    /// Note objects index sorted by its time.
    idx_by_time: BTreeMap<ObjTime, Vec<WavObjArenaIndex>>,
    /// The path of MIDI file, which is played as BGM while playing the score.
    #[cfg(feature = "minor-command")]
    pub midi_file: Option<PathBuf>,
    /// Material WAV file paths. #MATERIALSWAV
    #[cfg(feature = "minor-command")]
    pub materials_wav: Vec<PathBuf>,
    /// BGM volume change events, indexed by time. #97
    pub bgm_volume_changes: BTreeMap<ObjTime, BgmVolumeObj>,
    /// KEY volume change events, indexed by time. #98
    pub key_volume_changes: BTreeMap<ObjTime, KeyVolumeObj>,
    /// Seek events, indexed by time. #05
    #[cfg(feature = "minor-command")]
    pub seek_events: BTreeMap<ObjTime, SeekObj>,
    /// Text events, indexed by time. #99
    pub text_events: BTreeMap<ObjTime, TextObj>,
    /// Judge events, indexed by time. #A0
    pub judge_events: BTreeMap<ObjTime, JudgeObj>,
    /// BGA keybound events, indexed by time. #A5
    #[cfg(feature = "minor-command")]
    pub bga_keybound_events: BTreeMap<ObjTime, BgaKeyboundObj>,
    /// Option events, indexed by time. #A6
    #[cfg(feature = "minor-command")]
    pub option_events: BTreeMap<ObjTime, OptionObj>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for Notes<T> {
    fn default() -> Self {
        Self {
            wav_path_root: Default::default(),
            wav_files: Default::default(),
            arena: Default::default(),
            idx_by_wav_id: Default::default(),
            idx_by_channel: Default::default(),
            idx_by_time: Default::default(),
            #[cfg(feature = "minor-command")]
            midi_file: Default::default(),
            #[cfg(feature = "minor-command")]
            materials_wav: Default::default(),
            bgm_volume_changes: Default::default(),
            key_volume_changes: Default::default(),
            #[cfg(feature = "minor-command")]
            seek_events: Default::default(),
            text_events: Default::default(),
            judge_events: Default::default(),
            #[cfg(feature = "minor-command")]
            bga_keybound_events: Default::default(),
            #[cfg(feature = "minor-command")]
            option_events: Default::default(),
            _marker: PhantomData,
        }
    }
}

// query methods
impl<T> Notes<T> {
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
    pub fn all_notes(&self) -> impl Iterator<Item = &WavObj> {
        self.arena.0.iter().sorted()
    }

    /// Returns the iterator having all of the notes and its index sorted by time.
    pub fn all_entries(&self) -> impl Iterator<Item = (WavObjArenaIndex, &WavObj)> {
        self.arena
            .0
            .iter()
            .enumerate()
            .sorted_by_key(|obj| obj.1)
            .map(|(idx, obj)| (WavObjArenaIndex(idx), obj))
    }

    /// Returns all the playable notes in the score.
    pub fn playables(&self) -> impl Iterator<Item = &WavObj>
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
    pub fn displayables(&self) -> impl Iterator<Item = &WavObj>
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
    pub fn bgms(&self) -> impl Iterator<Item = &WavObj>
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
    pub fn notes_on(
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
            .map(|&arena_index| (arena_index, &self.arena.0[arena_index.0]))
    }

    /// Retrieves notes in the specified time span.
    pub fn notes_in<R: std::ops::RangeBounds<ObjTime>>(
        &self,
        time_span: R,
    ) -> impl DoubleEndedIterator<Item = (WavObjArenaIndex, &WavObj)> {
        self.idx_by_time
            .range(time_span)
            .flat_map(|(_, indexes)| indexes)
            .map(|&arena_index| (arena_index, &self.arena.0[arena_index.0]))
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
    pub fn last_playable_time(&self) -> Option<ObjTime>
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
    pub fn last_bgm_time(&self) -> Option<ObjTime>
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
impl<T> Notes<T> {
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
        if let Some(ids_by_channel_idx) = self.idx_by_channel[&channel_id]
            .iter()
            .position(|id| id.0 == idx)
        {
            self.idx_by_channel
                .get_mut(&channel_id)
                .expect("channel_id should exist in idx_by_channel")
                .swap_remove(ids_by_channel_idx);
        }
        if let Some(ids_by_time_idx) = self.idx_by_time[&removing.offset]
            .iter()
            .position(|id| id.0 == idx)
        {
            self.idx_by_time
                .get_mut(&removing.offset)
                .expect("offset should exist in idx_by_time")
                .swap_remove(ids_by_time_idx);
        }
    }

    /// Removes the latest note from the notes.
    pub fn pop_note(&mut self) -> Option<WavObj> {
        let last_idx = self.arena.0.len();
        let last = self.arena.0.pop()?;
        if let Some(ids_by_wav_id_idx) = self.idx_by_wav_id[&last.wav_id]
            .iter()
            .position(|id| id.0 == last_idx)
        {
            self.idx_by_wav_id
                .get_mut(&last.wav_id)?
                .swap_remove(ids_by_wav_id_idx);
        }
        let channel_id = last.channel_id;
        if let Some(ids_by_channel_idx) = self.idx_by_channel[&channel_id]
            .iter()
            .position(|id| id.0 == last_idx)
        {
            self.idx_by_channel
                .get_mut(&channel_id)?
                .swap_remove(ids_by_channel_idx);
        }
        if let Some(ids_by_time_idx) = self.idx_by_time[&last.offset]
            .iter()
            .position(|id| id.0 == last_idx)
        {
            self.idx_by_time
                .get_mut(&last.offset)?
                .swap_remove(ids_by_time_idx);
        }
        Some(last)
    }

    /// Removes notes belonging to the wav id.
    pub fn remove_note(&mut self, wav_id: ObjId) -> Vec<WavObj>
    where
        T: KeyLayoutMapper,
    {
        let Some(indexes) = self.idx_by_wav_id.remove(&wav_id) else {
            return vec![];
        };
        let mut objs = Vec::with_capacity(indexes.len());
        for WavObjArenaIndex(idx) in indexes {
            let removing = std::mem::replace(&mut self.arena.0[idx], WavObj::dangling());
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
    pub fn pop_latest_of(&mut self, wav_id: ObjId) -> Option<WavObj>
    where
        T: KeyLayoutMapper,
    {
        let &WavObjArenaIndex(to_pop) = self.idx_by_wav_id.get(&wav_id)?.last()?;
        let removing = std::mem::replace(&mut self.arena.0[to_pop], WavObj::dangling());
        self.remove_index(to_pop, &removing);
        Some(removing)
    }

    /// Adds the BGM (auto-played) note of `wav_id` at `time`.
    pub fn push_bgm(&mut self, time: ObjTime, wav_id: ObjId)
    where
        T: KeyLayoutMapper,
    {
        self.push_note(WavObj {
            offset: time,
            channel_id: NoteChannelId::bgm(),
            wav_id,
        });
    }

    /// Links the wav file `path` to the object id `wav_id`. Then returns the old path if existed.
    pub fn push_wav<P: Into<PathBuf>>(&mut self, wav_id: ObjId, path: P) -> Option<PathBuf> {
        self.wav_files.insert(wav_id, path.into())
    }

    /// Unlinks the wav file path from the object id `wav_id`, and return the path if existed.
    pub fn remove_wav(&mut self, wav_id: &ObjId) -> Option<PathBuf> {
        self.wav_files.remove(wav_id)
    }

    /// Retains note objects with the condition `cond`. It keeps only the [`WavObj`]s which `cond` returned `true`.
    pub fn retain_notes<F: FnMut(&WavObj) -> bool>(&mut self, mut cond: F)
    where
        T: KeyLayoutMapper,
    {
        let removing_indexes: Vec<_> = self
            .arena
            .0
            .iter()
            .enumerate()
            .filter(|&(_, obj)| cond(obj))
            .map(|(i, _)| i)
            .collect();
        for removing_idx in removing_indexes {
            let removing = std::mem::replace(&mut self.arena.0[removing_idx], WavObj::dangling());
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
            .map(|idx| &self.arena.0[idx.0])
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

    /// Adds a new BGM volume change object to the notes.
    pub fn push_bgm_volume_change(
        &mut self,
        volume_obj: BgmVolumeObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.bgm_volume_changes.entry(volume_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(volume_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::BgmVolumeChangeEvent {
                        time: volume_obj.time,
                        older: existing,
                        newer: &volume_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        volume_obj.clone(),
                        volume_obj.time,
                        Channel::BgmVolume,
                    )
            }
        }
    }

    /// Adds a new KEY volume change object to the notes.
    pub fn push_key_volume_change(
        &mut self,
        volume_obj: KeyVolumeObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.key_volume_changes.entry(volume_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(volume_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::KeyVolumeChangeEvent {
                        time: volume_obj.time,
                        older: existing,
                        newer: &volume_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        volume_obj.clone(),
                        volume_obj.time,
                        Channel::KeyVolume,
                    )
            }
        }
    }

    /// Adds a new seek object to the notes.
    #[cfg(feature = "minor-command")]
    pub fn push_seek_event(
        &mut self,
        seek_obj: SeekObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.seek_events.entry(seek_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(seek_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::SeekMessageEvent {
                        time: seek_obj.time,
                        older: existing,
                        newer: &seek_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        seek_obj.clone(),
                        seek_obj.time,
                        Channel::Seek,
                    )
            }
        }
    }

    /// Adds a new text object to the notes.
    pub fn push_text_event(
        &mut self,
        text_obj: TextObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.text_events.entry(text_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(text_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::TextEvent {
                        time: text_obj.time,
                        older: existing,
                        newer: &text_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        text_obj.clone(),
                        text_obj.time,
                        Channel::Text,
                    )
            }
        }
    }

    /// Adds a new judge object to the notes.
    pub fn push_judge_event(
        &mut self,
        judge_obj: JudgeObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.judge_events.entry(judge_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(judge_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::JudgeEvent {
                        time: judge_obj.time,
                        older: existing,
                        newer: &judge_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        judge_obj.clone(),
                        judge_obj.time,
                        Channel::Judge,
                    )
            }
        }
    }

    /// Adds a new BGA keybound object to the notes.
    #[cfg(feature = "minor-command")]
    pub fn push_bga_keybound_event(
        &mut self,
        keybound_obj: BgaKeyboundObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.bga_keybound_events.entry(keybound_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(keybound_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::BgaKeyboundEvent {
                        time: keybound_obj.time,
                        older: existing,
                        newer: &keybound_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        keybound_obj.clone(),
                        keybound_obj.time,
                        Channel::BgaKeybound,
                    )
            }
        }
    }

    /// Adds a new option object to the notes.
    #[cfg(feature = "minor-command")]
    pub fn push_option_event(
        &mut self,
        option_obj: OptionObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.option_events.entry(option_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(option_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::OptionEvent {
                        time: option_obj.time,
                        older: existing,
                        newer: &option_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        option_obj.clone(),
                        option_obj.time,
                        Channel::Option,
                    )
            }
        }
    }
}

// modify methods
impl<T> Notes<T> {
    /// Changes the channel of notes `target` in `time_span` into another channel `dst`.
    pub fn change_note_channel<I>(&mut self, targets: I, dst: NoteChannelId)
    where
        T: KeyLayoutMapper,
        I: IntoIterator<Item = WavObjArenaIndex>,
    {
        for target in targets {
            let Some(obj) = self.arena.0.get_mut(target.0) else {
                continue;
            };

            // Drain all ids from ids_by_channel where channel id matches
            let src = obj.channel_id;
            if let Some(idx_by_channel_idx) = self.idx_by_channel[&src]
                .iter()
                .position(|&idx| idx == target)
            {
                self.idx_by_channel
                    .get_mut(&src)
                    .expect("src channel_id should exist in idx_by_channel")
                    .swap_remove(idx_by_channel_idx);
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

        let idx_by_time = self.idx_by_time[&old_time]
            .iter()
            .position(|&idx| idx == target)?;
        self.idx_by_time
            .get_mut(&to_change.offset)?
            .swap_remove(idx_by_time);
        self.idx_by_time.entry(new_time).or_default().push(target);
        to_change.offset = new_time;
        Some(old_time)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use super::Notes;
    use crate::bms::prelude::*;

    #[test]
    fn push_and_pop() {
        let mut notes = Notes::<KeyLayoutBeat>::default();
        let note = WavObj {
            offset: ObjTime::new(
                1,
                2,
                NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
            ),
            channel_id: NoteChannelId::bgm(),
            wav_id: "01".try_into().unwrap(),
        };

        assert!(notes.pop_note().is_none());

        notes.push_note(note.clone());
        let removed = notes.pop_note();
        assert_eq!(Some(note), removed);

        assert!(notes.pop_note().is_none());
    }

    #[test]
    fn change_note_channel() {
        let mut notes = Notes::<KeyLayoutBeat>::default();
        let note = WavObj {
            offset: ObjTime::new(
                1,
                2,
                NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
            ),
            channel_id: NoteChannelId::bgm(),
            wav_id: "01".try_into().unwrap(),
        };

        assert!(notes.pop_note().is_none());

        notes.push_note(note.clone());
        let (idx, _) = notes.all_entries().next().unwrap();
        notes.change_note_channel(
            [idx],
            KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1)).to_channel_id(),
        );

        assert_eq!(
            notes.all_notes().next(),
            Some(&WavObj {
                offset: ObjTime::new(
                    1,
                    2,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64")
                ),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: "01".try_into().unwrap(),
            })
        );
    }

    #[test]
    fn change_note_time() {
        let mut notes = Notes::<KeyLayoutBeat>::default();
        let note = WavObj {
            offset: ObjTime::new(
                1,
                2,
                NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
            ),
            channel_id: NoteChannelId::bgm(),
            wav_id: "01".try_into().unwrap(),
        };

        assert!(notes.pop_note().is_none());

        notes.push_note(note.clone());
        let (idx, _) = notes.all_entries().next().unwrap();
        notes.change_note_time(
            idx,
            ObjTime::new(
                1,
                1,
                NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
            ),
        );

        assert_eq!(
            notes.all_notes().next(),
            Some(&WavObj {
                offset: ObjTime::new(
                    1,
                    1,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64")
                ),
                channel_id: NoteChannelId::bgm(),
                wav_id: "01".try_into().unwrap(),
            })
        );
    }
}
