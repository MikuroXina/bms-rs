use std::collections::{BTreeMap, HashSet};

use crate::{
    bms::prelude::*,
    parse::{
        Result,
        prompt::{ChannelDuplication, Prompter, TrackDuplication},
    },
};

use super::obj::{BpmChangeObj, SectionLenChangeObj};

/// The objects that arrange the playing panel running or showing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Arrangers {
    /// Section length change events, indexed by track. `#SECLEN`
    pub section_len_changes: BTreeMap<Track, SectionLenChangeObj>,
    /// The initial BPM of the score.
    pub bpm: Option<Decimal>,
    /// The BPMs corresponding to the id of the BPM change object.
    /// BPM change events, indexed by time. `#BPM[01-ZZ]` in message
    pub bpm_changes: BTreeMap<ObjTime, BpmChangeObj>,
    /// Record of used BPM change ids from `#BPMxx` messages, for validity checks.
    pub bpm_change_ids_used: HashSet<ObjId>,
    /// Stop lengths by stop object id.
    pub stops: BTreeMap<ObjTime, StopObj>,
    /// Record of used STOP ids from `#STOPxx` messages, for validity checks.
    pub stop_ids_used: HashSet<ObjId>,
    /// The scrolling factors corresponding to the id of the scroll speed change object.
    pub scrolling_factor_changes: BTreeMap<ObjTime, ScrollingFactorObj>,
    /// The spacing factors corresponding to the id of the spacing change object.
    pub speed_factor_changes: BTreeMap<ObjTime, SpeedObj>,
    /// bemaniaDX STP events, indexed by [`ObjTime`]. `#STP`
    #[cfg(feature = "minor-command")]
    pub stp_events: BTreeMap<ObjTime, StpEvent>,
    /// `#BASEBPM` for LR. Replaced by bpm match in LR2.
    #[cfg(feature = "minor-command")]
    pub base_bpm: Option<Decimal>,
}

impl Arrangers {
    /// Adds a new BPM change object to the notes.
    pub fn push_bpm_change(
        &mut self,
        bpm_change: BpmChangeObj,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        match self.bpm_changes.entry(bpm_change.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(bpm_change);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::BpmChangeEvent {
                        time: bpm_change.time,
                        older: existing,
                        newer: &bpm_change,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        bpm_change.clone(),
                        bpm_change.time,
                        Channel::BpmChange,
                    )
            }
        }
    }

    /// Adds a new scrolling factor change object to the notes.
    pub fn push_scrolling_factor_change(
        &mut self,
        scrolling_factor_change: ScrollingFactorObj,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        match self
            .scrolling_factor_changes
            .entry(scrolling_factor_change.time)
        {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(scrolling_factor_change);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::ScrollingFactorChangeEvent {
                        time: scrolling_factor_change.time,
                        older: existing,
                        newer: &scrolling_factor_change,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        scrolling_factor_change.clone(),
                        scrolling_factor_change.time,
                        Channel::Scroll,
                    )
            }
        }
    }

    /// Adds a new spacing factor change object to the notes.
    pub fn push_speed_factor_change(
        &mut self,
        speed_factor_change: SpeedObj,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        match self.speed_factor_changes.entry(speed_factor_change.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(speed_factor_change);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::SpeedFactorChangeEvent {
                        time: speed_factor_change.time,
                        older: existing,
                        newer: &speed_factor_change,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        speed_factor_change.clone(),
                        speed_factor_change.time,
                        Channel::Speed,
                    )
            }
        }
    }

    /// Adds a new section length change object to the notes.
    pub fn push_section_len_change(
        &mut self,
        section_len_change: SectionLenChangeObj,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        match self.section_len_changes.entry(section_len_change.track) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(section_len_change);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_track_duplication(TrackDuplication::SectionLenChangeEvent {
                        track: section_len_change.track,
                        older: existing,
                        newer: &section_len_change,
                    })
                    .apply_track(
                        entry.get_mut(),
                        section_len_change.clone(),
                        section_len_change.track,
                        Channel::SectionLen,
                    )
            }
        }
    }

    /// Adds a new stop object to the notes.
    pub fn push_stop(&mut self, stop: StopObj) {
        self.stops
            .entry(stop.time)
            .and_modify(|existing| {
                existing.duration = &existing.duration + &stop.duration;
            })
            .or_insert_with(|| stop);
    }
}
