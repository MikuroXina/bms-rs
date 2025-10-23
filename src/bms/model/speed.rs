use std::collections::{BTreeMap, HashMap, btree_map::Entry};

use crate::{
    bms::{error::Result, prelude::*},
    parse::prompt::ChannelDuplication,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpeedObjects {
    /// Spacing change definitions, indexed by [`ObjId`]. `#SPEED[01-ZZ]`
    pub speed_defs: HashMap<ObjId, Decimal>,
    /// The spacing factors corresponding to the id of the spacing change object.
    pub speed_factor_changes: BTreeMap<ObjTime, SpeedObj>,
}

impl SpeedObjects {
    /// Adds a new spacing factor change object to the notes.
    pub fn push_speed_factor_change(
        &mut self,
        speed_factor_change: SpeedObj,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        match self.speed_factor_changes.entry(speed_factor_change.time) {
            Entry::Vacant(entry) => {
                entry.insert(speed_factor_change);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
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
}
