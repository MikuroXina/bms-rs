//! This module introduces struct [`BmpObjects`], which manages definitions and events of BGA (Background Animation) and BGI (Background Image).
//!
//! BGA includes layers for base images, overlay images, and poor (miss) images. This module also handles
//! extended BGA commands like `#BGA`, `#@BGA`, `#SWBGA`, `#ARGB`, and opacity/color changes.

use std::{
    collections::{BTreeMap, HashMap, btree_map::Entry},
    path::PathBuf,
};

use crate::bms::{
    command::graphics::{Argb, PixelPoint, PixelSize},
    parse::{Result, prompt::ChannelDuplication},
    prelude::*,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// This aggregate manages definitions and events of BGA (Background Animation) and BGI (Background Image).
pub struct BmpObjects {
    /// The BMP file paths corresponding to the id of the background image/video object.
    pub bmp_files: HashMap<ObjId, Bmp>,
    /// The display mode for background image/video.
    pub poor_bga_mode: PoorMode,
    /// BGA change events, indexed by time. `#BGA`, `#BGAPOOR`, `#BGALAYER`
    pub bga_changes: BTreeMap<ObjTime, BgaObj>,
    /// The path of image, which is shown when the player got POOR.
    /// This image is displayed when the player misses a note or gets a poor judgment.
    pub poor_bmp: Option<PathBuf>,
    /// Storage for `#@BGAxx` definitions
    pub atbga_defs: HashMap<ObjId, AtBgaDef>,
    /// Storage for `#BGAxx` definitions
    pub bga_defs: HashMap<ObjId, BgaDef>,
    /// SWBGA events, indexed by [`ObjId`]. `#SWBGAxx`
    pub swbga_events: HashMap<ObjId, SwBgaEvent>,
    /// ARGB definitions, indexed by [`ObjId`]. `#ARGBxx`
    pub argb_defs: HashMap<ObjId, Argb>,
    /// BGA opacity change events, indexed by time. `#xxx0B:`, `#xxx0C:`, `#xxx0D:`, `#xxx0E:`
    pub bga_opacity_changes: HashMap<BgaLayer, BTreeMap<ObjTime, BgaOpacityObj>>,
    /// BGA color change events, indexed by time. `#xxxA1:`, `#xxxA2:`, `#xxxA3:`, `#xxxA4:`
    pub bga_argb_changes: HashMap<BgaLayer, BTreeMap<ObjTime, BgaArgbObj>>,
    /// BGA keybound events, indexed by time. `#xxxA5:`
    pub bga_keybound_events: BTreeMap<ObjTime, BgaKeyboundObj>,
}

impl BmpObjects {
    /// Gets the time of the last BGA change object.
    #[must_use]
    pub fn last_obj_time(&self) -> Option<ObjTime> {
        self.bga_changes.last_key_value().map(|(&time, _)| time)
    }
}

impl BmpObjects {
    /// Adds a new bga change object to the notes.
    ///
    /// # Errors
    ///
    /// Returns [`ParseWarning`] if a conflict is found and the
    /// provided [`Prompter`] decides to treat it as an error.
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
    ///
    /// # Errors
    ///
    /// Returns [`ParseWarning`] if a conflict is found and the
    /// provided [`Prompter`] decides to treat it as an error.
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
    ///
    /// # Errors
    ///
    /// Returns [`ParseWarning`] if a conflict is found and the
    /// provided [`Prompter`] decides to treat it as an error.
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
    ///
    /// # Errors
    ///
    /// Returns [`ParseWarning`] if a conflict is found and the
    /// provided [`Prompter`] decides to treat it as an error.
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

/// A background image/video data.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bmp {
    /// The path to the image/video file. This is relative path from the BMS file.
    pub file: PathBuf,
    /// The color which should to be treated as transparent. It should be used only if `file` is an image.
    pub transparent_color: Argb,
}

/// A definition for `#@BGA` command.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AtBgaDef {
    /// The object ID.
    pub id: ObjId,
    /// The source BMP object ID.
    pub source_bmp: ObjId,
    /// The top-left position for trimming in pixels.
    pub trim_top_left: PixelPoint,
    /// The size for trimming in pixels.
    pub trim_size: PixelSize,
    /// The draw point position in pixels.
    pub draw_point: PixelPoint,
}

/// A definition for `#BGA` command.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BgaDef {
    /// The object ID.
    pub id: ObjId,
    /// The source BMP object ID.
    pub source_bmp: ObjId,
    /// The top-left position for trimming in pixels.
    pub trim_top_left: PixelPoint,
    /// The bottom-right position for trimming in pixels.
    pub trim_bottom_right: PixelPoint,
    /// The draw point position in pixels.
    pub draw_point: PixelPoint,
}
