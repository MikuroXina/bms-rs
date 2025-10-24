use std::{
    collections::{BTreeMap, HashMap, btree_map::Entry},
    path::PathBuf,
};

use crate::{
    bms::{error::Result, prelude::*},
    parse::prompt::ChannelDuplication,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BmpObjects {
    /// The BMP file paths corresponding to the id of the background image/video object.
    pub bmp_files: HashMap<ObjId, Bmp>,
    /// The display mode for background image/video.
    pub poor_bga_mode: PoorMode,
    /// BGA change events, indexed by time. #BGA, #BGAPOOR, #BGALAYER
    pub bga_changes: BTreeMap<ObjTime, BgaObj>,
    /// The path of image, which is shown when the player got POOR.
    /// This image is displayed when the player misses a note or gets a poor judgment.
    pub poor_bmp: Option<PathBuf>,
    /// Storage for #@BGA definitions

    pub atbga_defs: HashMap<ObjId, AtBgaDef>,
    /// Storage for #BGA definitions

    pub bga_defs: HashMap<ObjId, BgaDef>,
    /// SWBGA events, indexed by [`ObjId`]. `#SWBGA`

    pub swbga_events: HashMap<ObjId, SwBgaEvent>,
    /// ARGB definitions, indexed by [`ObjId`]. `#ARGB`

    pub argb_defs: HashMap<ObjId, Argb>,
    /// BGA opacity change events, indexed by time. #0B, #0C, #0D, #0E

    pub bga_opacity_changes: HashMap<BgaLayer, BTreeMap<ObjTime, BgaOpacityObj>>,
    /// BGA ARGB color change events, indexed by time. #A1, #A2, #A3, #A4

    pub bga_argb_changes: HashMap<BgaLayer, BTreeMap<ObjTime, BgaArgbObj>>,
    /// BGA keybound events, indexed by time. #A5

    pub bga_keybound_events: BTreeMap<ObjTime, BgaKeyboundObj>,
}

impl BmpObjects {
    /// Gets the time of the last BPM change object.
    #[must_use]
    pub fn last_obj_time(&self) -> Option<ObjTime> {
        self.bga_changes.last_key_value().map(|(&time, _)| time)
    }
}

impl BmpObjects {
    /// Adds a new bga change object to the notes.
    pub fn push_bga_change(
        &mut self,
        bga: BgaObj,
        channel: Channel,
        prompter: &impl Prompter,
    ) -> Result<()> {
        match self.bga_changes.entry(bga.time) {
            Entry::Vacant(entry) => {
                entry.insert(bga);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompter
                    .handle_channel_duplication(ChannelDuplication::BgaChangeEvent {
                        time: bga.time,
                        older: existing,
                        newer: &bga,
                    })
                    .apply_channel(entry.get_mut(), bga, bga.time, channel)
            }
        }
    }

    /// Adds a new BGA opacity change object to the graphics.

    pub fn push_bga_opacity_change(
        &mut self,
        opacity_obj: BgaOpacityObj,
        channel: Channel,
        prompter: &impl Prompter,
    ) -> Result<()> {
        let this_layer_map = self
            .bga_opacity_changes
            .entry(opacity_obj.layer)
            .or_default();
        match this_layer_map.entry(opacity_obj.time) {
            Entry::Vacant(entry) => {
                entry.insert(opacity_obj);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompter
                    .handle_channel_duplication(ChannelDuplication::BgaOpacityChangeEvent {
                        time: opacity_obj.time,
                        older: existing,
                        newer: &opacity_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        opacity_obj.clone(),
                        opacity_obj.time,
                        channel,
                    )
            }
        }
    }

    /// Adds a new BGA ARGB color change object to the graphics.

    pub fn push_bga_argb_change(
        &mut self,
        argb_obj: BgaArgbObj,
        channel: Channel,
        prompter: &impl Prompter,
    ) -> Result<()> {
        let this_layer_map = self.bga_argb_changes.entry(argb_obj.layer).or_default();
        match this_layer_map.entry(argb_obj.time) {
            Entry::Vacant(entry) => {
                entry.insert(argb_obj);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompter
                    .handle_channel_duplication(ChannelDuplication::BgaArgbChangeEvent {
                        time: argb_obj.time,
                        older: existing,
                        newer: &argb_obj,
                    })
                    .apply_channel(entry.get_mut(), argb_obj.clone(), argb_obj.time, channel)
            }
        }
    }

    /// Adds a new BGA keybound object to the notes.

    pub fn push_bga_keybound_event(
        &mut self,
        keybound_obj: BgaKeyboundObj,
        prompter: &impl Prompter,
    ) -> Result<()> {
        match self.bga_keybound_events.entry(keybound_obj.time) {
            Entry::Vacant(entry) => {
                entry.insert(keybound_obj);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompter
                    .handle_channel_duplication(ChannelDuplication::BgaKeyboundEvent {
                        time: keybound_obj.time,
                        older: existing,
                        newer: &keybound_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        keybound_obj.clone(),
                        keybound_obj.time,
                        Channel::BgaKeybound,
                    )
            }
        }
    }
}
