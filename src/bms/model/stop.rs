//! This module introduces struct [`StopObjects`], which manages definitions and events of scroll stop.

use std::collections::{BTreeMap, HashMap, HashSet, btree_map::Entry};

use strict_num_extended::NonNegativeF64;

use crate::bms::parse::{
    Result,
    prompt::{ChannelDuplication, Prompter},
};
use crate::bms::{
    command::{StringValue, channel::Channel},
    prelude::*,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// This aggregate manages definitions and events of scroll stop.
pub struct StopObjects {
    /// Stop definitions, indexed by [`ObjId`]. `#STOP[01-ZZ]`
    pub stop_defs: HashMap<ObjId, StringValue<NonNegativeF64>>,
    /// Stop lengths by stop object id.
    pub stops: BTreeMap<ObjTime, StopObj>,
    /// Record of used STOP ids from `#STOPxx` messages, for validity checks.
    pub stop_ids_used: HashSet<ObjId>,
    /// bemaniaDX STP events, indexed by [`ObjTime`]. `#STP`
    pub stp_events: BTreeMap<ObjTime, StpEvent>,
}

impl StopObjects {
    /// Gets the time of the last STOP object.
    #[must_use]
    pub fn last_obj_time(&self) -> Option<ObjTime> {
        self.stops.last_key_value().map(|(&time, _)| time)
    }
}

impl StopObjects {
    /// Adds a new stop object to the notes.
    ///
    /// # Errors
    ///
    /// Returns [`ParseWarning`] if a conflict is found and
    /// provided [`Prompter`] decides to treat it as an error.
    pub fn push_stop(&mut self, stop: StopObj, prompt_handler: &impl Prompter) -> Result<()> {
        match self.stops.entry(stop.time) {
            Entry::Vacant(entry) => {
                entry.insert(stop);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::StopEvent {
                        time: stop.time,
                        older: existing,
                        newer: &stop,
                    })
                    .apply_channel(entry.get_mut(), stop.clone(), stop.time, Channel::Stop)
            }
        }
    }

    /// Adds a new stop object to the notes, ignoring any duplicates.
    pub fn push_stop_ignore_duplicate(&mut self, stop: StopObj) {
        self.stops.insert(stop.time, stop);
    }
}
