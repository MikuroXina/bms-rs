//! This module introduces struct [`OptionObjects`], which manages vendor-specific BMS player configuration and that events.

use crate::bms::{error::Result, prelude::*};
use std::collections::{BTreeMap, HashMap, btree_map::Entry};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// This aggregate manages vendor-specific BMS player configuration and that events.
pub struct OptionObjects {
    /// The message for overriding options of some BMS player.
    pub options: Option<Vec<String>>,
    /// The option messages corresponding to the id of the change option object.
    pub change_options: HashMap<ObjId, String>,
    /// Option events, indexed by time. #A6
    pub option_events: BTreeMap<ObjTime, OptionObj>,
}

impl OptionObjects {
    /// Adds a new option object to the notes.
    pub fn push_option_event(
        &mut self,
        option_obj: OptionObj,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        match self.option_events.entry(option_obj.time) {
            Entry::Vacant(entry) => {
                entry.insert(option_obj);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
                use super::super::parse::prompt::ChannelDuplication;

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
                        Channel::OptionChange,
                    )
            }
        }
    }
}
