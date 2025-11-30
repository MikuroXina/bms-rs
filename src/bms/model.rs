//! Header information from parsed BMS file.
//! Note objects manager.

pub mod bmp;
pub mod bpm;
pub mod control_flow;
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
    bmp::BmpObjects, bpm::BpmObjects, control_flow::RandomizedObjects, judge::JudgeObjects,
    metadata::Metadata, music_info::MusicInfo, repr::BmsSourceRepresentation,
    scroll::ScrollObjects, section_len::SectionLenObjects, speed::SpeedObjects, sprite::Sprites,
    stop::StopObjects, text::TextObjects, video::Video, volume::VolumeObjects, wav::WavObjects,
};

use self::{option::OptionObjects, resources::Resources};

/// A score data aggregate of BMS format.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bms {
    /// Manager of background image animation.
    pub bmp: BmpObjects,
    /// Manager of BPM events.
    pub bpm: BpmObjects,
    /// Manager of judgments to score plays.
    pub judge: JudgeObjects,
    /// Manager of data to organize BMS scores in your player.
    pub metadata: Metadata,
    /// Manager of the music information.
    pub music_info: MusicInfo,
    /// Manager of vendor-specific configurations.
    pub option: OptionObjects,
    /// Manager about representation format of the BMS source.
    pub repr: BmsSourceRepresentation,
    /// Manager of external resource paths.
    pub resources: Resources,
    /// Manager of scroll speed change events.
    pub scroll: ScrollObjects,
    /// Manager of section length change events.
    pub section_len: SectionLenObjects,
    /// Manager of spacing factor change events.
    pub speed: SpeedObjects,
    /// Manager of image assets except BGA/BGI.
    pub sprite: Sprites,
    /// Manager of scroll stop events.
    pub stop: StopObjects,
    /// Manager of caption events.
    pub text: TextObjects,
    /// Manager of background video.
    pub video: Video,
    /// Manager of volume controls.
    pub volume: VolumeObjects,
    /// Manager of sounds.
    pub wav: WavObjects,
    /// Manager of randomized control flow.
    pub randomized: Vec<RandomizedObjects>,
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

    /// Merge another Bms object into this one, returning a new Bms.
    ///
    /// Fields from `other` overwrite `self` if they are present.
    /// Collections are extended.
    #[must_use]
    pub fn union(&self, other: Bms) -> Self {
        let mut res = self.clone();
        res.union_inplace(&other);
        res
    }

    /// Merge another Bms object into this one in-place.
    ///
    /// Fields from `other` overwrite `self` if they are present.
    /// Collections are extended.
    pub fn union_inplace(&mut self, other: &Bms) {
        // bmp
        self.bmp.bmp_files.extend(other.bmp.bmp_files.clone());
        self.bmp.bga_changes.extend(other.bmp.bga_changes.clone());
        if other.bmp.poor_bmp.is_some() {
            self.bmp.poor_bmp = other.bmp.poor_bmp.clone();
        }
        self.bmp.atbga_defs.extend(other.bmp.atbga_defs.clone());
        self.bmp.bga_defs.extend(other.bmp.bga_defs.clone());
        self.bmp.swbga_events.extend(other.bmp.swbga_events.clone());
        self.bmp.argb_defs.extend(other.bmp.argb_defs.clone());
        for (k, v) in &other.bmp.bga_opacity_changes {
            self.bmp
                .bga_opacity_changes
                .entry(*k)
                .or_default()
                .extend(v.clone());
        }
        for (k, v) in &other.bmp.bga_argb_changes {
            self.bmp
                .bga_argb_changes
                .entry(*k)
                .or_default()
                .extend(v.clone());
        }
        self.bmp
            .bga_keybound_events
            .extend(other.bmp.bga_keybound_events.clone());

        // bpm
        self.bpm.bpm_changes.extend(other.bpm.bpm_changes.clone());
        if other.bpm.bpm.is_some() {
            self.bpm.bpm = other.bpm.bpm.clone();
        }
        if other.bpm.base_bpm.is_some() {
            self.bpm.base_bpm = other.bpm.base_bpm.clone();
        }
        self.bpm.bpm_defs.extend(other.bpm.bpm_defs.clone());
        self.bpm
            .bpm_changes_u8
            .extend(other.bpm.bpm_changes_u8.clone());

        // judge
        if other.judge.rank.is_some() {
            self.judge.rank = other.judge.rank;
        }
        if other.judge.total.is_some() {
            self.judge.total = other.judge.total.clone();
        }
        self.judge
            .exrank_defs
            .extend(other.judge.exrank_defs.clone());
        self.judge
            .judge_events
            .extend(other.judge.judge_events.clone());

        // metadata
        if other.metadata.player.is_some() {
            self.metadata.player = other.metadata.player;
        }
        if other.metadata.play_level.is_some() {
            self.metadata.play_level = other.metadata.play_level;
        }
        if other.metadata.difficulty.is_some() {
            self.metadata.difficulty = other.metadata.difficulty;
        }
        if other.metadata.email.is_some() {
            self.metadata.email = other.metadata.email.clone();
        }
        if other.metadata.url.is_some() {
            self.metadata.url = other.metadata.url.clone();
        }
        if other.metadata.wav_path_root.is_some() {
            self.metadata.wav_path_root = other.metadata.wav_path_root.clone();
        }
        if other.metadata.divide_prop.is_some() {
            self.metadata.divide_prop = other.metadata.divide_prop.clone();
        }
        // is_octave is bool, so we can't know if it was "set" or default false.
        // We'll assume if other is true, we take it.
        if other.metadata.is_octave {
            self.metadata.is_octave = true;
        }

        // music_info
        if other.music_info.genre.is_some() {
            self.music_info.genre = other.music_info.genre.clone();
        }
        if other.music_info.title.is_some() {
            self.music_info.title = other.music_info.title.clone();
        }
        if other.music_info.subtitle.is_some() {
            self.music_info.subtitle = other.music_info.subtitle.clone();
        }
        if other.music_info.artist.is_some() {
            self.music_info.artist = other.music_info.artist.clone();
        }
        if other.music_info.sub_artist.is_some() {
            self.music_info.sub_artist = other.music_info.sub_artist.clone();
        }
        if other.music_info.maker.is_some() {
            self.music_info.maker = other.music_info.maker.clone();
        }
        if other.music_info.comment.is_some() {
            self.music_info.comment = other.music_info.comment.clone();
        }
        if other.music_info.preview_music.is_some() {
            self.music_info.preview_music = other.music_info.preview_music.clone();
        }

        // option
        if other.option.options.is_some() {
            self.option.options = other.option.options.clone();
        }
        self.option
            .change_options
            .extend(other.option.change_options.clone());
        self.option
            .option_events
            .extend(other.option.option_events.clone());

        // repr
        if other.repr.ln_type != Default::default() {
            self.repr.ln_type = other.repr.ln_type;
        }
        if other.repr.ln_mode != Default::default() {
            self.repr.ln_mode = other.repr.ln_mode;
        }
        if other.repr.charset.is_some() {
            self.repr.charset = other.repr.charset.clone();
        }
        self.repr
            .raw_command_lines
            .extend(other.repr.raw_command_lines.clone());
        self.repr
            .non_command_lines
            .extend(other.repr.non_command_lines.clone());
        if other.repr.case_sensitive_obj_id {
            self.repr.case_sensitive_obj_id = true;
        }

        // resources
        if other.resources.midi_file.is_some() {
            self.resources.midi_file = other.resources.midi_file.clone();
        }
        self.resources.cdda.extend(other.resources.cdda.clone());
        self.resources
            .materials_wav
            .extend(other.resources.materials_wav.clone());
        self.resources
            .materials_bmp
            .extend(other.resources.materials_bmp.clone());
        if other.resources.materials_path.is_some() {
            self.resources.materials_path = other.resources.materials_path.clone();
        }

        // scroll
        self.scroll
            .scrolling_factor_changes
            .extend(other.scroll.scrolling_factor_changes.clone());
        self.scroll
            .scroll_defs
            .extend(other.scroll.scroll_defs.clone());

        // section_len
        self.section_len
            .section_len_changes
            .extend(other.section_len.section_len_changes.clone());

        // speed
        self.speed
            .speed_factor_changes
            .extend(other.speed.speed_factor_changes.clone());
        self.speed.speed_defs.extend(other.speed.speed_defs.clone());

        // sprite
        if other.sprite.back_bmp.is_some() {
            self.sprite.back_bmp = other.sprite.back_bmp.clone();
        }
        if other.sprite.stage_file.is_some() {
            self.sprite.stage_file = other.sprite.stage_file.clone();
        }
        if other.sprite.banner.is_some() {
            self.sprite.banner = other.sprite.banner.clone();
        }
        self.sprite
            .extchr_events
            .extend(other.sprite.extchr_events.clone());
        if other.sprite.char_file.is_some() {
            self.sprite.char_file = other.sprite.char_file.clone();
        }

        // stop
        self.stop.stop_defs.extend(other.stop.stop_defs.clone());
        self.stop.stp_events.extend(other.stop.stp_events.clone());
        for stop in other.stop.stops.values() {
            self.stop.push_stop(stop.clone());
        }

        // text
        self.text.text_events.extend(other.text.text_events.clone());

        // video
        if other.video.video_file.is_some() {
            self.video.video_file = other.video.video_file.clone();
        }
        if other.video.video_colors.is_some() {
            self.video.video_colors = other.video.video_colors;
        }
        if other.video.video_dly.is_some() {
            self.video.video_dly = other.video.video_dly.clone();
        }
        if other.video.video_fs.is_some() {
            self.video.video_fs = other.video.video_fs.clone();
        }
        self.video.seek_defs.extend(other.video.seek_defs.clone());
        self.video
            .seek_events
            .extend(other.video.seek_events.clone());

        // volume
        self.volume
            .bgm_volume_changes
            .extend(other.volume.bgm_volume_changes.clone());
        self.volume
            .key_volume_changes
            .extend(other.volume.key_volume_changes.clone());

        // wav
        self.wav.wav_files.extend(other.wav.wav_files.clone());
        for note in other.wav.notes.all_notes() {
            self.wav.notes.push_note(note.clone());
        }

        // randomized
        self.randomized.extend(other.randomized.clone());
    }
}
