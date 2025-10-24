use std::collections::{BTreeMap, HashMap, HashSet, btree_map::Entry};

use crate::{
    bms::{error::Result, prelude::*},
    parse::prompt::ChannelDuplication,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BpmObjects {
    /// The initial BPM of the score.
    pub bpm: Option<Decimal>,
    /// BPM change definitions, indexed by [`ObjId`]. `#BPM[01-ZZ]`
    pub bpm_defs: HashMap<ObjId, Decimal>,
    /// `#BASEBPM` for LR. Replaced by bpm match in LR2.

    pub base_bpm: Option<Decimal>,
    /// The BPMs corresponding to the id of the BPM change object.
    /// BPM change events, indexed by time. `#BPM[01-ZZ]` in message
    pub bpm_changes: BTreeMap<ObjTime, BpmChangeObj>,
    /// BPM change events on its channel [`Channel::BpmChangeU8`], indexed by time.
    pub bpm_changes_u8: BTreeMap<ObjTime, u8>,
    /// Record of used BPM change ids from `#BPMxx` messages, for validity checks.
    pub bpm_change_ids_used: HashSet<ObjId>,
}

impl BpmObjects {
    /// Gets the time of the last BPM change object.
    #[must_use]
    pub fn last_obj_time(&self) -> Option<ObjTime> {
        self.bpm_changes.last_key_value().map(|(&time, _)| time)
    }

    /// Calculates a required resolution to convert the notes time into pulses, which split one quarter note evenly.
    #[must_use]
    pub fn resolution_for_pulses(&self) -> u64 {
        use num::Integer;

        let mut hyp_resolution = 1;
        for bpm_change in self.bpm_changes.values() {
            hyp_resolution = hyp_resolution.lcm(&bpm_change.time.denominator().get());
        }
        hyp_resolution
    }
}

impl BpmObjects {
    /// Adds a new BPM change object to the notes.
    pub fn push_bpm_change(
        &mut self,
        bpm_change: BpmChangeObj,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        match self.bpm_changes.entry(bpm_change.time) {
            Entry::Vacant(entry) => {
                entry.insert(bpm_change);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
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

    /// Adds a new BPM change (on [`Channel::BpmChangeU8`] channel) object to the notes.
    pub fn push_bpm_change_u8(
        &mut self,
        time: ObjTime,
        bpm_change: u8,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        match self.bpm_changes_u8.entry(time) {
            Entry::Vacant(entry) => {
                entry.insert(bpm_change);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
                let existing = entry.get();
                let older = BpmChangeObj {
                    time,
                    bpm: Decimal::from(*existing),
                };
                let newer = BpmChangeObj {
                    time,
                    bpm: Decimal::from(bpm_change),
                };
                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::BpmChangeEvent {
                        time,
                        older: &older,
                        newer: &newer,
                    })
                    .apply_channel(entry.get_mut(), bpm_change, time, Channel::BpmChangeU8)
            }
        }
    }
}
