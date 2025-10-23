//! Header information from parsed BMS file.
//! Note objects manager.

pub mod bmp;
pub mod bpm;
pub mod def;
pub mod judge;
pub mod metadata;
pub mod music_info;
mod notes;
pub mod obj;
pub mod option;
pub mod repr;
pub mod resources;
pub mod scroll;
pub mod section_len;
pub mod speed;
pub mod sprite;
pub mod stop;
pub mod text;
pub mod video;
pub mod volume;
pub mod wav;

use std::fmt::Debug;

use crate::bms::command::time::ObjTime;

pub use notes::Notes;

use self::{
    bmp::BmpObjects, bpm::BpmObjects, judge::JudgeObjects, metadata::Metadata,
    music_info::MusicInfo, option::OptionObjects, repr::BmsSourceRepresentation,
    resources::Resources, scroll::ScrollObjects, section_len::SectionLenObjects,
    speed::SpeedObjects, sprite::Sprites, stop::StopObjects, text::TextObjects, video::Video,
    volume::VolumeObjects, wav::WavObjects,
};

/// A score data aggregate of BMS format.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bms {
    pub bmp: BmpObjects,
    pub bpm: BpmObjects,
    pub judge: JudgeObjects,
    pub metadata: Metadata,
    pub music_info: MusicInfo,
    #[cfg(feature = "minor-command")]
    pub option: OptionObjects,
    pub repr: BmsSourceRepresentation,
    #[cfg(feature = "minor-command")]
    pub resources: Resources,
    pub scroll: ScrollObjects,
    pub section_len: SectionLenObjects,
    pub speed: SpeedObjects,
    pub sprite: Sprites,
    pub stop: StopObjects,
    pub text: TextObjects,
    pub video: Video,
    pub volume: VolumeObjects,
    pub wav: WavObjects,
}

impl Bms {
    /// Returns the sound note objects information.
    #[must_use]
    pub const fn notes(&self) -> &Notes {
        &self.wav.notes
    }

    /// Gets the time of last any object including visible, BGM, BPM change, section length change and so on.
    ///
    /// You can't use this to find the length of music. Because this doesn't consider that the length of sound.
    #[must_use]
    pub fn last_obj_time(&self) -> Option<ObjTime> {
        let obj_last = self.wav.notes.last_obj_time();
        let bpm_last = self.bpm.last_obj_time();
        let section_len_last = self.section_len.last_obj_time();
        let stop_last = self.stop.last_obj_time();
        let bga_last = self.bmp.last_obj_time();
        [obj_last, bpm_last, section_len_last, stop_last, bga_last]
            .into_iter()
            .max()
            .flatten()
    }

    /// Calculates a required resolution to convert the notes time into pulses, which split one quarter note evenly.
    #[must_use]
    pub fn resolution_for_pulses(&self) -> u64 {
        use num::Integer;

        let mut hyp_resolution = 1u64;
        for obj in self.wav.notes.all_notes() {
            hyp_resolution = hyp_resolution.lcm(&obj.offset.denominator().get());
        }
        hyp_resolution
    }
}
