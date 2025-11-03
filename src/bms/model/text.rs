//! This module introduces struct [`TextObjects`], which manages definitions and events of caption texts.

use std::collections::{BTreeMap, HashMap, btree_map::Entry};

use crate::bms::{parse::Result, parse::prompt::ChannelDuplication, prelude::*};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// This aggregate manages definitions and events of caption texts.
pub struct TextObjects {
    /// Storage for `#TEXT` definitions
    /// The texts corresponding to the id of the text object.
    pub texts: HashMap<ObjId, String>,
    /// Text events, indexed by time. `#xxx99:`
    pub text_events: BTreeMap<ObjTime, TextObj>,
}

impl TextObjects {
    /// Adds a new text object to the notes.
    pub fn push_text_event(
        &mut self,
        text_obj: TextObj,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        match self.text_events.entry(text_obj.time) {
            Entry::Vacant(entry) => {
                entry.insert(text_obj);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::TextEvent {
                        time: text_obj.time,
                        older: existing,
                        newer: &text_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        text_obj.clone(),
                        text_obj.time,
                        Channel::Text,
                    )
            }
        }
    }
}
