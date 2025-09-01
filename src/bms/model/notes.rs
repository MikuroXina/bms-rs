use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap},
    marker::PhantomData,
    ops::Bound,
    path::PathBuf,
};

use itertools::Itertools;

use crate::{
    bms::prelude::*,
    command::channel::ChannelId,
    parse::{Result, prompt::ChannelDuplication},
};

/// The playable objects set for querying by lane or time.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Notes<T> {
    /// The path to override the base path of the WAV file path.
    /// This allows WAV files to be referenced relative to a different directory.
    pub wav_path_root: Option<PathBuf>,
    /// The WAV file paths corresponding to the id of the note object.
    pub wav_files: HashMap<ObjId, PathBuf>,
    // objects stored in obj is sorted, so it can be searched by bisection method
    /// BGM objects, indexed by time. `#XXX01:ZZ...` (BGM placement)
    pub bgms: BTreeMap<ObjTime, Vec<ObjId>>,
    /// All note objects, indexed by [`ObjId`]. `#XXXYY:ZZ...` (note placement)
    pub objs: HashMap<ObjId, Vec<WavObj>>,
    /// Index for fast key lookup. Used for LN/landmine logic and so on.
    /// Maps each [`ChannelId`] to a sorted map of times and [`ObjId`]s for efficient note lookup.
    pub ids_by_channel: HashMap<ChannelId, BTreeMap<ObjTime, ObjId>>,
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
            bgms: Default::default(),
            objs: Default::default(),
            ids_by_channel: Default::default(),
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

impl<T> Notes<T> {
    /// Converts into the notes sorted by time.
    #[must_use]
    pub fn into_all_notes(self) -> Vec<WavObj> {
        self.objs.into_values().flatten().sorted().collect()
    }

    /// Returns the iterator having all of the notes sorted by time.
    pub fn all_notes(&self) -> impl Iterator<Item = &WavObj> {
        self.objs.values().flatten().sorted()
    }

    /// Returns all the bgms in the score.
    #[must_use]
    pub const fn bgms(&self) -> &BTreeMap<ObjTime, Vec<ObjId>> {
        &self.bgms
    }

    /// Retrieves notes on the specified channel id by the key mapping `T`.
    pub fn notes_on(&self, channel_id: ChannelId) -> impl Iterator<Item = &WavObj>
    where
        T: KeyLayoutMapper,
    {
        self.objs
            .values()
            .flatten()
            .filter(move |obj| T::new(obj.side, obj.kind, obj.key).to_channel_id() == channel_id)
    }

    /// Retrieves notes in the specified time span.
    pub fn notes_in<R: std::ops::RangeBounds<ObjTime>>(
        &self,
        time_span: R,
    ) -> impl Iterator<Item = &WavObj> {
        self.objs
            .values()
            .flatten()
            .filter(move |obj| time_span.contains(&obj.offset))
    }
}

impl<T> Notes<T> {
    /// Finds next object on the key `Key` from the time `ObjTime`.
    #[must_use]
    pub fn next_obj_by_key(
        &self,
        side: PlayerSide,
        kind: NoteKind,
        key: Key,
        time: ObjTime,
    ) -> Option<&WavObj>
    where
        T: KeyLayoutMapper,
    {
        let id = T::new(side, kind, key).to_channel_id();
        self.ids_by_channel
            .get(&id)?
            .range((Bound::Excluded(time), Bound::Unbounded))
            .next()
            .and_then(|(_, id)| {
                let objs = self.objs.get(id)?;
                let idx = objs
                    .binary_search_by(|probe| probe.offset.cmp(&time))
                    .unwrap_or_else(|idx| idx);
                objs.get(idx)
            })
    }

    /// Adds the new note object to the notes.
    pub fn push_note(&mut self, note: WavObj)
    where
        T: KeyLayoutMapper,
    {
        let entry_key = T::new(note.side, note.kind, note.key).to_channel_id();
        let offset = note.offset;
        let obj = note.obj;
        self.objs.entry(obj).or_default().push(note);
        self.ids_by_channel
            .entry(entry_key)
            .or_default()
            .insert(offset, obj);
    }

    /// Removes the latest note from the notes.
    pub fn remove_latest_note(&mut self, id: ObjId) -> Option<WavObj>
    where
        T: KeyLayoutMapper,
    {
        self.objs.entry(id).or_default().pop().inspect(|removed| {
            let entry_key = T::new(removed.side, removed.kind, removed.key).to_channel_id();
            if let Some(key_map) = self.ids_by_channel.get_mut(&entry_key) {
                key_map.remove(&removed.offset);
            }
        })
    }

    /// Removes the note from the notes.
    pub fn remove_note(&mut self, id: ObjId) -> Vec<WavObj>
    where
        T: KeyLayoutMapper,
    {
        self.objs.remove(&id).map_or(vec![], |removed| {
            for item in &removed {
                let entry_key = T::new(item.side, item.kind, item.key).to_channel_id();
                if let Some(key_map) = self.ids_by_channel.get_mut(&entry_key) {
                    key_map.remove(&item.offset);
                }
            }
            removed
        })
    }

    /// Gets the time of last visible object.
    pub fn last_visible_time(&self) -> Option<ObjTime> {
        self.objs
            .values()
            .flatten()
            .filter(|obj| !matches!(obj.kind, NoteKind::Invisible))
            .map(Reverse)
            .sorted()
            .next()
            .map(|Reverse(obj)| obj.offset)
    }

    /// Gets the time of last BGM object.
    ///
    /// You can't use this to find the length of music. Because this doesn't consider that the length of sound. And visible notes may ring after all BGMs.
    #[must_use]
    pub fn last_bgm_time(&self) -> Option<ObjTime> {
        self.bgms.last_key_value().map(|(time, _)| time).cloned()
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
