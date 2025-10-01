use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use crate::{
    bms::prelude::*,
    parse::{Result, prompt::ChannelDuplication},
};

/// The graphics objects that are used in the score.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Graphics {
    /// The path of the background video. The video should be started the playing from the section 000.
    /// This video is displayed behind the gameplay area.
    /// Its audio track should not be played.
    pub video_file: Option<PathBuf>,
    /// The BMP file paths corresponding to the id of the background image/video object.
    pub bmp_files: HashMap<ObjId, Bmp>,
    /// BGA change events, indexed by time. #BGA, #BGAPOOR, #BGALAYER
    pub bga_changes: BTreeMap<ObjTime, BgaObj>,
    /// The path of image, which is shown when the player got POOR.
    /// This image is displayed when the player misses a note or gets a poor judgment.
    pub poor_bmp: Option<PathBuf>,
    /// The display mode for background image/video.
    pub poor_bga_mode: PoorMode,
    /// Material BMP file paths. #MATERIALSBMP
    #[cfg(feature = "minor-command")]
    pub materials_bmp: Vec<PathBuf>,
    /// Character file path. #CHARFILE
    #[cfg(feature = "minor-command")]
    pub char_file: Option<PathBuf>,
    /// Video color depth. #VIDEOCOLORS
    #[cfg(feature = "minor-command")]
    pub video_colors: Option<u8>,
    /// Video delay. #VIDEODLY
    #[cfg(feature = "minor-command")]
    pub video_dly: Option<Decimal>,
    /// Video frame rate. #VIDEOF/S
    #[cfg(feature = "minor-command")]
    pub video_fs: Option<Decimal>,
    /// BGA opacity change events, indexed by time. #0B, #0C, #0D, #0E
    #[cfg(feature = "minor-command")]
    pub bga_opacity_changes: HashMap<BgaLayer, BTreeMap<ObjTime, BgaOpacityObj>>,
    /// BGA ARGB color change events, indexed by time. #A1, #A2, #A3, #A4
    #[cfg(feature = "minor-command")]
    pub bga_argb_changes: HashMap<BgaLayer, BTreeMap<ObjTime, BgaArgbObj>>,
}

impl Graphics {
    /// Returns the bga change objects.
    #[must_use]
    pub const fn bga_changes(&self) -> &BTreeMap<ObjTime, BgaObj> {
        &self.bga_changes
    }

    /// Adds a new bga change object to the notes.
    pub fn push_bga_change(
        &mut self,
        bga: BgaObj,
        channel: Channel,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        match self.bga_changes.entry(bga.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(bga);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
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
    #[cfg(feature = "minor-command")]
    pub fn push_bga_opacity_change(
        &mut self,
        opacity_obj: BgaOpacityObj,
        channel: Channel,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        let this_layer_map = self
            .bga_opacity_changes
            .entry(opacity_obj.layer)
            .or_default();
        match this_layer_map.entry(opacity_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(opacity_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
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
    #[cfg(feature = "minor-command")]
    pub fn push_bga_argb_change(
        &mut self,
        argb_obj: BgaArgbObj,
        channel: Channel,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        let this_layer_map = self.bga_argb_changes.entry(argb_obj.layer).or_default();
        match this_layer_map.entry(argb_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(argb_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::BgaArgbChangeEvent {
                        time: argb_obj.time,
                        older: existing,
                        newer: &argb_obj,
                    })
                    .apply_channel(entry.get_mut(), argb_obj.clone(), argb_obj.time, channel)
            }
        }
    }
}
