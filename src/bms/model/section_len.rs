//! This module introduces struct [`SectionLenObjects`], which manages events of section length change.

use std::collections::{BTreeMap, btree_map::Entry};

use crate::bms::{
    parse::{Result, prompt::TrackDuplication},
    prelude::*,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// This aggregate manages events of section length change.
pub struct SectionLenObjects {
    /// Section length change events, indexed by track. `#SECLEN`
    pub section_len_changes: BTreeMap<Track, SectionLenChangeObj>,
}

impl SectionLenObjects {
    /// Gets the time of the last section length change.
    #[must_use]
    pub fn last_obj_time(&self) -> Option<ObjTime> {
        self.section_len_changes
            .last_key_value()
            .map(|(&time, _)| ObjTime::start_of(time))
    }
}

impl SectionLenObjects {
    /// Adds a new section length change object to the notes.
    ///
    /// # Errors
    ///
    /// Returns [`ParseWarning`](crate::bms::parse::ParseWarning) if a conflict is found and the
    /// provided [`Prompter`] decides to treat it as an error.
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
