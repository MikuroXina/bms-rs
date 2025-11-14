//! This module introduces struct [`VolumeObjects`], which manages volume control and that events.

use std::collections::{BTreeMap, btree_map::Entry};

use crate::bms::{
    parse::{Result, prompt::ChannelDuplication},
    prelude::*,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// This aggregate manages volume control and that events.
pub struct VolumeObjects {
    /// The volume of the score.
    pub volume: Volume,
    /// BGM volume change events, indexed by time. `#xxx97:`
    pub bgm_volume_changes: BTreeMap<ObjTime, BgmVolumeObj>,
    /// KEY volume change events, indexed by time. `#xxx98:`
    pub key_volume_changes: BTreeMap<ObjTime, KeyVolumeObj>,
}

impl VolumeObjects {
    /// Adds a new BGM volume change object to the notes.
    pub fn push_bgm_volume_change(
        &mut self,
        volume_obj: BgmVolumeObj,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        match self.bgm_volume_changes.entry(volume_obj.time) {
            Entry::Vacant(entry) => {
                entry.insert(volume_obj);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
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
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        match self.key_volume_changes.entry(volume_obj.time) {
            Entry::Vacant(entry) => {
                entry.insert(volume_obj);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
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
}
