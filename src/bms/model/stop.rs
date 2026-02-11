//! This module introduces struct [`StopObjects`], which manages definitions and events of scroll stop.

use std::collections::{BTreeMap, HashMap, HashSet};

use strict_num_extended::NonNegativeF64;

use crate::bms::{command::StringValue, prelude::*};

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
    pub fn push_stop(&mut self, stop: StopObj) {
        self.stops
            .entry(stop.time)
            .and_modify(|existing| {
                let sum = existing.duration.as_f64() + stop.duration.as_f64();
                existing.duration = NonNegativeF64::new(sum).unwrap_or(NonNegativeF64::ZERO);
            })
            .or_insert_with(|| stop);
    }
}
