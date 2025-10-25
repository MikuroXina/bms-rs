//! This module introduces struct [`ScrollObjects`], which manages definitions and events of scroll speed change.

use std::collections::{BTreeMap, HashMap, btree_map::Entry};

use crate::{
    bms::{error::Result, prelude::*},
    parse::prompt::ChannelDuplication,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// This aggregate manages definition and events of scroll speed change.
pub struct ScrollObjects {
    /// Scroll speed change definitions, indexed by [`ObjId`]. `#SCROLL[01-ZZ]`
    pub scroll_defs: HashMap<ObjId, Decimal>,
    /// The scrolling factors corresponding to the id of the scroll speed change object.
    pub scrolling_factor_changes: BTreeMap<ObjTime, ScrollingFactorObj>,
}

impl ScrollObjects {
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
            Entry::Vacant(entry) => {
                entry.insert(scrolling_factor_change);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
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
}
