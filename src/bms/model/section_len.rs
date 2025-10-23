use std::{
    collections::{BTreeMap, btree_map::Entry},
    num::NonZeroU64,
};

use crate::{
    bms::{error::Result, prelude::*},
    parse::prompt::TrackDuplication,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SectionLenObjects {
    /// Section length change events, indexed by track. `#SECLEN`
    pub section_len_changes: BTreeMap<Track, SectionLenChangeObj>,
}

impl SectionLenObjects {
    /// Gets the time of the last section length change.
    #[must_use]
    pub fn last_obj_time(&self) -> Option<ObjTime> {
        self.section_len_changes.last_key_value().map(|(&time, _)| {
            ObjTime::new(
                time.0,
                0,
                NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
            )
        })
    }
}

impl SectionLenObjects {
    /// Adds a new section length change object to the notes.
    pub fn push_section_len_change(
        &mut self,
        section_len_change: SectionLenChangeObj,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        match self.section_len_changes.entry(section_len_change.track) {
            Entry::Vacant(entry) => {
                entry.insert(section_len_change);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
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
}
