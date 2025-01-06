//! The [bmson format](https://bmson-spec.readthedocs.io/en/master/doc/index.html) definition.
//!
//! # Order of Processing
//!
//! When there are coincident events in the same pulse, they are processed in the order below:
//!
//! - [`Note`] and [`BgaEvent`] (are independent each other),
//! - [`BpmEvent`],
//! - [`StopEvent`].
//!
//! If a [`BpmEvent`] and a [`StopEvent`] appear on the same pulse, the current BPM will be changed at first, then scrolling the chart will be stopped for a while depending the changed BPM.
//!
//! If a [`Note`] and a [`StopEvent`] appear on the same pulse, the sound will be played (or should be hit by a player), then scrolling the chart will be stopped.
//!
//! # Layered Notes
//!
//! In case that notes (not BGM) from different sound channels exist on the same (key and pulse) position:
//!
//! - When its length is not equal to each other, yo should treat as an error and warn to a player.
//! - Otherwise your player may fusion the notes. That means when a player hit the key, two sounds will be played.
//!
//! # Differences from BMS
//!
//! - BMS can play different sound on the start and end of long note. But bmson does not allow this.
//! - Transparent color on BGA is not supported. But you can use PNG files having RGBA channels.

use std::{collections::HashMap, num::NonZeroU8};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    lex::command::{JudgeLevel, Key},
    parse::{notes::BgaLayer, Bms},
    time::{ObjTime, Track},
};

use self::{
    fin_f64::FinF64,
    pulse::{PulseConverter, PulseNumber},
};

pub mod fin_f64;
pub mod pulse;

/// Top-level object for bmson format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bmson {
    /// Version of bmson format, which should be compared using [Semantic Version 2.0.0](http://semver.org/spec/v2.0.0.html). Older bmson file may not have this field, but lacking this must be an error.
    pub version: String,
    /// Score metadata.
    pub info: BmsonInfo,
    /// Location of bar lines in pulses. If `None`, then a 4/4 beat is assumed and bar lines will be generates every 4 quarter notes. If `Some(vec![])`, this chart will not have any bar line.
    ///
    /// This format represents an irregular meter by bar lines.
    pub lines: Option<Vec<BarLine>>,
    /// Events of bpm change. If there are coincident events, the successor is only applied.
    #[serde(default)]
    pub bpm_events: Vec<BpmEvent>,
    /// Events of scroll stop. If there are coincident events, they are happened in succession.
    #[serde(default)]
    pub stop_events: Vec<StopEvent>,
    /// Note data.
    pub sound_channels: Vec<SoundChannel>,
    /// BGA data.
    pub bga: Bga,
}

/// Header metadata of chart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BmsonInfo {
    /// Self explanatory title.
    pub title: String,
    /// Self explanatory subtitle. Usually this is shown as a smaller text than `title`.
    #[serde(default)]
    pub subtitle: String,
    /// Author of the chart. It may multiple names such as `Alice vs Bob`, `Alice feat. Bob` and so on. But you should respect the value because it usually have special meaning.
    pub artist: String,
    /// Other authors of the chart. This is useful for indexing and searching.
    ///
    /// Value of the array has form of `key:value`. The `key` can be `music`, `vocal`, `chart`, `image`, `movie` or `other`. If it has no `key`, you should treat as that `key` equals to `other`. The value may contains the spaces before and after `key` and `value`, so you should trim them.
    ///
    /// # Example
    ///
    /// ```json
    /// "subartists": ["music:5argon", "music:encX", "chart:flicknote", "movie:5argon", "image:5argon"]
    /// ```
    #[serde(default)]
    pub subartists: Vec<String>,
    /// Self explanatory genre.
    pub genre: String,
    /// Hint for layout lanes, e.g. "beat-7k", "popn-5k", "generic-nkeys". Defaults to `"beat-7k"`.
    ///
    /// If you want to support many lane modes of BMS, you should check this to determine the layout for lanes. Also you can check all lane information in `sound_channels` for strict implementation.
    #[serde(default = "default_mode_hint")]
    pub mode_hint: String,
    /// Special chart name, e.g. "BEGINNER", "NORMAL", "HYPER", "FOUR DIMENSIONS".
    #[serde(default)]
    pub chart_name: String,
    /// Self explanatory level number. It is usually set with subjective rating by the author.
    pub level: u32,
    /// Initial BPM.
    pub init_bpm: FinF64,
    /// Relative judge width in percentage. The variation amount may different by BMS player. Larger is easier.
    #[serde(default = "default_percentage")]
    pub judge_rank: FinF64,
    /// Relative life bar gain in percentage. The variation amount may different by BMS player. Larger is easier.
    #[serde(default = "default_percentage")]
    pub total: FinF64,
    /// Background image file name. This should be displayed during the game play.
    pub back_image: Option<String>,
    /// Eyecatch image file name. This should be displayed during the chart is loading.
    pub eyecatch_image: Option<String>,
    /// Title image file name. This should be displayed before the game starts instead of title of the music.
    pub title_image: Option<String>,
    /// Banner image file name. This should be displayed in music select or result scene. The aspect ratio of image is usually 15:4.
    pub banner_image: Option<String>,
    /// Preview music file name. This should be played when this chart is selected in a music select scene.
    pub preview_music: Option<String>,
    /// Numbers of pulse per quarter note in 4/4 measure. You must check this because it affects the actual seconds of `PulseNumber`.
    #[serde(default = "default_resolution")]
    pub resolution: u32,
}

/// Default mode hint, beatmania 7 keys.
pub fn default_mode_hint() -> String {
    "beat-7k".into()
}

/// Default relative percentage, 100%.
pub fn default_percentage() -> FinF64 {
    FinF64::new(100.0).unwrap()
}

/// Default resolution pulses per quarter note in 4/4 measure, 240 pulses.
pub fn default_resolution() -> u32 {
    240
}

/// Event of bar line of the chart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BarLine {
    /// Pulse number to place the line.
    pub y: PulseNumber,
}

/// Note sound file and positions to be placed in the chart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundChannel {
    /// Sound file path. If the extension is not specified or not supported, you can try search files about other extensions for fallback.
    ///
    /// BMS players are expected to support the audio containers below:
    ///
    /// - WAV (`.wav`),
    /// - OGG (`.ogg`),
    /// - Audio-only MPEG-4 (`.m4a`).
    ///
    /// BMS players are expected to support the audio codec below:
    ///
    /// - LPCM (Linear Pulse-Code Modulation),
    /// - Ogg Vorbis,
    /// - AAC (Advanced Audio Coding).
    pub name: String,
    /// Data of note to be placed.
    pub notes: Vec<Note>,
}

/// Sound note to ring a sound file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// Lane information. The `Some` number represents the key to play, otherwise it is not playable (BGM) note.
    pub x: Option<NonZeroU8>,
    /// Position to be placed.
    pub y: PulseNumber,
    /// Length of pulses of the note. It will be a normal note if zero, otherwise a long note.
    pub l: u32,
    /// Continuation flag. It will continue to ring rest of the file when play if `true`, otherwise it will play from start.
    pub c: bool,
}

/// BPM change note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BpmEvent {
    /// Position to change BPM of the chart.
    pub y: PulseNumber,
    /// New BPM to be.
    pub bpm: FinF64,
}

/// Scroll stop note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopEvent {
    /// Start position to scroll stop.
    pub y: PulseNumber,
    /// Stopping duration in pulses.
    pub duration: u32,
}

/// BGA data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bga {
    /// Pictures data for playing BGA.
    pub bga_header: Vec<BgaHeader>,
    /// Base picture sequence.
    pub bga_events: Vec<BgaEvent>,
    /// Layered picture sequence.
    pub layer_events: Vec<BgaEvent>,
    /// Picture sequence displayed when missed.
    pub poor_events: Vec<BgaEvent>,
}

/// Picture file information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BgaHeader {
    /// Self explanatory ID of picture.
    pub id: BgaId,
    /// Picture file name.
    pub name: String,
}

/// BGA note to display the picture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BgaEvent {
    /// Position to display the picture in pulses.
    pub y: PulseNumber,
    /// ID of picture to display.
    pub id: BgaId,
}

/// Picture id for [`Bga`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BgaId(pub u32);

/// Errors on converting from `Bms` into `Bmson`.
#[derive(Debug, Error, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BmsonConvertError {
    /// The initial BPM was infinity or NaN.
    #[error("header bpm was invalid value")]
    InvalidBpm,
    /// The total percentage was infinity or NaN.
    #[error("header total was invalid value")]
    InvalidTotal,
}

impl TryFrom<Bms> for Bmson {
    type Error = BmsonConvertError;

    fn try_from(value: Bms) -> Result<Self, Self::Error> {
        let converter = PulseConverter::new(&value.notes);

        let has_7keys = value
            .notes
            .all_notes()
            .any(|note| note.key.is_extended_key());

        const EASY_WIDTH: f64 = 21.0;
        const NORMAL_WIDTH: f64 = 18.0;
        const HARD_WIDTH: f64 = 15.0;
        const VERY_HARD_WIDTH: f64 = 8.0;
        let judge_rank = FinF64::new(match value.header.rank {
            Some(JudgeLevel::Easy) => EASY_WIDTH / NORMAL_WIDTH,
            Some(JudgeLevel::Normal) | None => 1.0,
            Some(JudgeLevel::Hard) => HARD_WIDTH / NORMAL_WIDTH,
            Some(JudgeLevel::VeryHard) => VERY_HARD_WIDTH / NORMAL_WIDTH,
        })
        .unwrap();

        let resolution = value.notes.resolution_for_pulses();

        let last_obj_time = value
            .notes
            .last_obj_time()
            .unwrap_or_else(|| ObjTime::new(0, 0, 4));
        let lines = (0..=last_obj_time.track.0)
            .map(|track| BarLine {
                y: converter.get_pulses_on(Track(track)),
            })
            .collect();

        let bpm_events = value
            .notes
            .bpm_changes()
            .values()
            .map(|bpm_change| {
                Ok(BpmEvent {
                    y: converter.get_pulses_at(bpm_change.time),
                    bpm: FinF64::new(bpm_change.bpm).ok_or(BmsonConvertError::InvalidBpm)?,
                })
            })
            .collect::<Result<Vec<_>, BmsonConvertError>>()?;

        let stop_events = value
            .notes
            .stops()
            .values()
            .map(|stop| StopEvent {
                y: converter.get_pulses_at(stop.time),
                duration: stop.duration,
            })
            .collect();

        let info = BmsonInfo {
            title: value.header.title.unwrap_or_default(),
            subtitle: value.header.subtitle.unwrap_or_default(),
            artist: value.header.artist.unwrap_or_default(),
            subartists: vec![value.header.sub_artist.unwrap_or_default()],
            genre: value.header.genre.unwrap_or_default(),
            mode_hint: if has_7keys {
                "beat-7k".into()
            } else {
                "beat-5k".into()
            },
            chart_name: "".into(),
            level: value.header.play_level.unwrap_or_default() as u32,
            init_bpm: FinF64::new(value.header.bpm.unwrap_or(120.0))
                .ok_or(BmsonConvertError::InvalidBpm)?,
            judge_rank,
            total: FinF64::new(value.header.total.unwrap_or(100.0))
                .ok_or(BmsonConvertError::InvalidTotal)?,
            back_image: value
                .header
                .back_bmp
                .as_ref()
                .cloned()
                .map(|path| path.display().to_string()),
            eyecatch_image: value
                .header
                .stage_file
                .map(|path| path.display().to_string()),
            title_image: value.header.back_bmp.map(|path| path.display().to_string()),
            banner_image: value.header.banner.map(|path| path.display().to_string()),
            preview_music: None,
            resolution,
        };

        let sound_channels = {
            let path_root = value.header.wav_path_root.unwrap_or_default();
            let mut sound_channels = HashMap::new();
            for note in value.notes.all_notes() {
                let note_lane = note
                    .kind
                    .is_playable()
                    .then_some(
                        match note.key {
                            Key::Key1 => 1,
                            Key::Key2 => 2,
                            Key::Key3 => 3,
                            Key::Key4 => 4,
                            Key::Key5 => 5,
                            Key::Key6 => 6,
                            Key::Key7 => 7,
                            Key::Scratch | Key::FreeZone => 8,
                        } + if note.is_player1 { 0 } else { 8 },
                    )
                    .map(|num| NonZeroU8::new(num).unwrap());
                let pulses = converter.get_pulses_at(note.offset);
                let duration =
                    if let Some(next_note) = value.notes.next_obj_by_key(note.key, note.offset) {
                        pulses.abs_diff(converter.get_pulses_at(next_note.offset))
                    } else {
                        0
                    };
                let to_insert = Note {
                    x: note_lane,
                    y: pulses,
                    l: duration,
                    c: false,
                };

                sound_channels
                    .entry(note.obj)
                    .and_modify(|channel: &mut SoundChannel| channel.notes.push(to_insert.clone()))
                    .or_insert_with(|| {
                        let sound_path = path_root.join(
                            value
                                .header
                                .wav_files
                                .get(&note.obj)
                                .cloned()
                                .unwrap_or_default(),
                        );
                        SoundChannel {
                            name: sound_path.display().to_string(),
                            notes: vec![to_insert],
                        }
                    });
            }
            sound_channels.into_values().collect()
        };

        let bga = {
            let mut bga = Bga {
                bga_header: vec![],
                bga_events: vec![],
                layer_events: vec![],
                poor_events: vec![],
            };
            for (id, bmp) in &value.header.bmp_files {
                bga.bga_header.push(BgaHeader {
                    id: BgaId(id.as_u32()),
                    name: bmp.file.display().to_string(),
                });
            }
            for (&time, change) in value.notes.bga_changes() {
                let target = match change.layer {
                    BgaLayer::Base => &mut bga.bga_events,
                    BgaLayer::Poor => &mut bga.poor_events,
                    BgaLayer::Overlay => &mut bga.layer_events,
                };
                target.push(BgaEvent {
                    y: converter.get_pulses_at(time),
                    id: BgaId(change.id.as_u32()),
                })
            }
            bga
        };

        Ok(Self {
            version: "1.0.0".into(),
            info,
            lines: Some(lines),
            bpm_events,
            stop_events,
            sound_channels,
            bga,
        })
    }
}
