#[cfg(feature = "minor-command")]
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

#[cfg(feature = "minor-command")]
use crate::bms::error::Result;
use crate::bms::prelude::*;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Video {
    /// Movie Define. Defines the global video file for the chart.
    /// - Video starts from section #000
    /// - Priority rules apply when conflicting with #xxx04
    /// - No loop, stays on last frame after playback
    /// - Audio track in video is not played
    pub video_file: Option<PathBuf>,
    /// Video color depth. #VIDEOCOLORS
    #[cfg(feature = "minor-command")]
    pub video_colors: Option<u8>,
    /// Video delay. #VIDEODLY
    #[cfg(feature = "minor-command")]
    pub video_dly: Option<Decimal>,
    /// Video frame rate. #VIDEOF/S
    #[cfg(feature = "minor-command")]
    pub video_fs: Option<Decimal>,
    /// Seek event definitions. `#SEEK`
    #[cfg(feature = "minor-command")]
    pub seek_defs: HashMap<ObjId, Decimal>,
    /// Seek events, indexed by time. `#05`
    #[cfg(feature = "minor-command")]
    pub seek_events: BTreeMap<ObjTime, SeekObj>,
}

impl Video {
    /// Adds a new seek object to the notes.
    #[cfg(feature = "minor-command")]
    pub fn push_seek_event(
        &mut self,
        seek_obj: SeekObj,
        prompt_handler: &impl Prompter,
    ) -> Result<()> {
        use std::collections::btree_map::Entry;

        match self.seek_events.entry(seek_obj.time) {
            Entry::Vacant(entry) => {
                entry.insert(seek_obj);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
                use crate::parse::prompt::ChannelDuplication;

                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::SeekMessageEvent {
                        time: seek_obj.time,
                        older: existing,
                        newer: &seek_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        seek_obj.clone(),
                        seek_obj.time,
                        Channel::Seek,
                    )
            }
        }
    }
}
