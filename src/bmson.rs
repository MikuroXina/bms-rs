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

use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::{
    lex::command::{JudgeLevel, Key, NoteKind, PlayerSide},
    parse::{Bms, notes::BgaLayer},
    time::{ObjTime, Track},
};

use self::{
    fin_f64::FinF64,
    pulse::{PulseConverter, PulseNumber},
};

use crate::bms::Decimal;

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
    #[serde(default)]
    pub bga: Bga,
    /// Beatoraja implementation of scroll events.
    #[serde(default)]
    pub scroll_events: Vec<ScrollEvent>,
    /// Beatoraja implementation of mine channel.
    #[serde(default)]
    pub mine_channels: Vec<MineChannel>,
    /// Beatoraja implementation of invisible key channel.
    #[serde(default)]
    pub key_channels: Vec<KeyChannel>,
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
    #[serde(default)]
    pub back_image: Option<String>,
    /// Eyecatch image file name. This should be displayed during the chart is loading.
    #[serde(default)]
    pub eyecatch_image: Option<String>,
    /// Title image file name. This should be displayed before the game starts instead of title of the music.
    #[serde(default)]
    pub title_image: Option<String>,
    /// Banner image file name. This should be displayed in music select or result scene. The aspect ratio of image is usually 15:4.
    #[serde(default)]
    pub banner_image: Option<String>,
    /// Preview music file name. This should be played when this chart is selected in a music select scene.
    #[serde(default)]
    pub preview_music: Option<String>,
    /// Numbers of pulse per quarter note in 4/4 measure. You must check this because it affects the actual seconds of `PulseNumber`.
    #[serde(default = "default_resolution")]
    pub resolution: u64,
    /// Beatoraja implementation of long note type.
    #[serde(default)]
    pub ln_type: LongNoteType,
}

/// Default mode hint, beatmania 7 keys.
pub fn default_mode_hint() -> String {
    "beat-7k".into()
}

/// Default relative percentage, 100%.
pub fn default_percentage() -> FinF64 {
    FinF64::new(100.0).expect("Internal error: 100.0 is not a valid FinF64")
}

/// Default resolution pulses per quarter note in 4/4 measure, 240 pulses.
pub fn default_resolution() -> u64 {
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
    /// Position to be placed.
    pub y: PulseNumber,
    /// Lane information. The `Some` number represents the key to play, otherwise it is not playable (BGM) note.
    #[serde(deserialize_with = "deserialize_x_none_if_zero")]
    pub x: Option<NonZeroU8>,
    /// Length of pulses of the note. It will be a normal note if zero, otherwise a long note.
    pub l: u64,
    /// Continuation flag. It will continue to ring rest of the file when play if `true`, otherwise it will play from start.
    pub c: bool,
    /// Beatoraja implementation of long note type.
    #[serde(default)]
    pub t: LongNoteType,
    /// Beatoraja implementation of long note up flag.
    /// If it is true and configured at the end position of a long note, then this position will become the ending note of the long note.
    #[serde(default)]
    pub up: bool,
}

fn deserialize_x_none_if_zero<'de, D>(deserializer: D) -> Result<Option<NonZeroU8>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<u8>::deserialize(deserializer)?;
    Ok(match opt {
        Some(0) => None,
        Some(v) => NonZeroU8::new(v),
        None => None,
    })
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
    pub duration: u64,
}

/// BGA data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Bga {
    /// Pictures data for playing BGA.
    #[serde(default)]
    pub bga_header: Vec<BgaHeader>,
    /// Base picture sequence.
    #[serde(default)]
    pub bga_events: Vec<BgaEvent>,
    /// Layered picture sequence.
    #[serde(default)]
    pub layer_events: Vec<BgaEvent>,
    /// Picture sequence displayed when missed.
    #[serde(default)]
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

/// Beatoraja implementation of long note type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum LongNoteType {
    /// Normal long note.
    #[default]
    LN = 1,
    /// Continuous long note.
    CN = 2,
    /// Hell continuous long note.
    HCN = 3,
}

/// Beatoraja implementation of scroll event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScrollEvent {
    /// Position to scroll.
    pub y: PulseNumber,
    /// Scroll rate.
    pub rate: FinF64,
}

/// Beatoraja implementation of mine channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MineEvent {
    /// Lane information. The `Some` number represents the key to play, otherwise it is not playable (BGM) note.
    #[serde(deserialize_with = "deserialize_x_none_if_zero")]
    pub x: Option<NonZeroU8>,
    /// Position to be placed.
    pub y: PulseNumber,
    /// Damage of the mine.
    pub damage: FinF64,
}

/// Beatoraja implementation of mine channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MineChannel {
    /// Name of the mine sound file.
    pub name: String,
    /// Mine notes.
    pub notes: Vec<MineEvent>,
}

/// Beatoraja implementation of invisible key event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEvent {
    /// Lane information. The `Some` number represents the key to play, otherwise it is not playable (BGM) note.
    #[serde(deserialize_with = "deserialize_x_none_if_zero")]
    pub x: Option<NonZeroU8>,
    /// Position to be placed.
    pub y: PulseNumber,
}

/// Beatoraja implementation of invisible key channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyChannel {
    /// Name of the key sound file.
    pub name: String,
    /// Invisible key notes.
    pub notes: Vec<KeyEvent>,
}

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
    /// The scrolling factor was infinity or NaN.
    #[error("scrolling factor was invalid value")]
    InvalidScrollingFactor,
    /// The judge rank was infinity or NaN.
    #[error("judge rank was invalid value")]
    InvalidJudgeRank,
    /// The stop duration was infinity or NaN.
    #[error("stop duration was invalid value")]
    InvalidStopDuration,
    /// The note lane was invalid.
    #[error("note lane was invalid value")]
    InvalidNoteLane,
}

impl TryFrom<Bms> for Bmson {
    type Error = BmsonConvertError;

    fn try_from(value: Bms) -> Result<Self, Self::Error> {
        let converter = PulseConverter::new(&value.notes);

        const EASY_WIDTH: f64 = 21.0;
        const VERY_EASY_WIDTH: f64 = EASY_WIDTH * 1.25;
        const NORMAL_WIDTH: f64 = 18.0;
        const HARD_WIDTH: f64 = 15.0;
        const VERY_HARD_WIDTH: f64 = 8.0;
        let judge_rank = FinF64::new(match value.header.rank {
            Some(JudgeLevel::OtherInt(4)) => VERY_EASY_WIDTH / NORMAL_WIDTH, // VeryEasy implementation of beatoraja.
            Some(JudgeLevel::Easy) => EASY_WIDTH / NORMAL_WIDTH,
            Some(JudgeLevel::Normal) | None => 1.0,
            Some(JudgeLevel::Hard) => HARD_WIDTH / NORMAL_WIDTH,
            Some(JudgeLevel::VeryHard) => VERY_HARD_WIDTH / NORMAL_WIDTH,
            Some(JudgeLevel::OtherInt(_)) => 1.0,
        })
        .expect("Internal error: judge rank is invalid");

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
                    bpm: FinF64::new(
                        bpm_change
                            .bpm
                            .clone()
                            .try_into()
                            .map_err(|_| BmsonConvertError::InvalidBpm)?,
                    )
                    .ok_or(BmsonConvertError::InvalidBpm)?,
                })
            })
            .collect::<Result<Vec<_>, BmsonConvertError>>()?;

        let stop_events = value
            .notes
            .stops()
            .values()
            .map(|stop| {
                Ok(StopEvent {
                    y: converter.get_pulses_at(stop.time),
                    duration: stop
                        .duration
                        .clone()
                        .try_into()
                        .map_err(|_| BmsonConvertError::InvalidStopDuration)?,
                })
            })
            .collect::<Result<Vec<_>, BmsonConvertError>>()?;

        let info = BmsonInfo {
            title: value.header.title.unwrap_or_default(),
            subtitle: value.header.subtitle.unwrap_or_default(),
            artist: value.header.artist.unwrap_or_default(),
            subartists: vec![value.header.sub_artist.unwrap_or_default()],
            genre: value.header.genre.unwrap_or_default(),
            mode_hint: {
                // TODO: Support other modes
                let is_7keys = value
                    .notes
                    .all_notes()
                    .any(|note| note.key == Key::Key6 || note.key == Key::Key7);
                let is_dp = value
                    .notes
                    .all_notes()
                    .any(|note| note.side == PlayerSide::Player2);
                match (is_dp, is_7keys) {
                    (true, true) => "beat-14k".into(),
                    (true, false) => "beat-10k".into(),
                    (false, true) => "beat-7k".into(),
                    (false, false) => "beat-5k".into(),
                }
            },
            chart_name: "".into(),
            level: value.header.play_level.unwrap_or_default() as u32,
            init_bpm: FinF64::new(
                value
                    .header
                    .bpm
                    .unwrap_or(Decimal::from(120.0))
                    .try_into()
                    .map_err(|_| BmsonConvertError::InvalidBpm)?,
            )
            .ok_or(BmsonConvertError::InvalidBpm)?,
            judge_rank: FinF64::new(
                judge_rank
                    .try_into()
                    .map_err(|_| BmsonConvertError::InvalidJudgeRank)?,
            )
            .ok_or(BmsonConvertError::InvalidJudgeRank)?,
            total: FinF64::new(
                value
                    .header
                    .total
                    .unwrap_or(Decimal::from(100.0))
                    .try_into()
                    .map_err(|_| BmsonConvertError::InvalidTotal)?,
            )
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
            ln_type: LongNoteType::LN,
        };

        let (sound_channels, mine_channels, key_channels) = {
            let path_root = value.header.wav_path_root.clone().unwrap_or_default();
            let mut sound_map: HashMap<_, Vec<Note>> = HashMap::new();
            let mut mine_map: HashMap<_, Vec<MineEvent>> = HashMap::new();
            let mut key_map: HashMap<_, Vec<KeyEvent>> = HashMap::new();
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
                            // TODO: Extra key convertion
                            Key::Key8
                            | Key::Key9
                            | Key::Key10
                            | Key::Key11
                            | Key::Key12
                            | Key::Key13
                            | Key::Key14
                            | Key::ScratchExtra
                            | Key::FootPedal => 0,
                        } + match note.side {
                            PlayerSide::Player1 => 0,
                            PlayerSide::Player2 => 8,
                        },
                    )
                    .map(|lane| NonZeroU8::new(lane).ok_or(BmsonConvertError::InvalidNoteLane))
                    .transpose()?;
                let pulses = converter.get_pulses_at(note.offset);
                match note.kind {
                    NoteKind::Landmine => {
                        let damage = FinF64::new(100.0)
                            .expect("Internal error: 100.0 is not a valid FinF64");
                        mine_map.entry(note.obj).or_default().push(MineEvent {
                            x: note_lane,
                            y: pulses,
                            damage,
                        });
                    }
                    NoteKind::Invisible => {
                        key_map.entry(note.obj).or_default().push(KeyEvent {
                            x: note_lane,
                            y: pulses,
                        });
                    }
                    _ => {
                        // Normal note
                        let duration = if let Some(next_note) =
                            value.notes.next_obj_by_key(note.key, note.offset)
                        {
                            pulses.abs_diff(converter.get_pulses_at(next_note.offset))
                        } else {
                            0
                        };
                        sound_map.entry(note.obj).or_default().push(Note {
                            x: note_lane,
                            y: pulses,
                            l: duration,
                            c: false,
                            t: LongNoteType::LN,
                            up: false,
                        });
                    }
                }
            }
            let sound_channels = sound_map
                .into_iter()
                .map(|(obj, notes)| {
                    let sound_path = path_root.join(
                        value
                            .header
                            .wav_files
                            .get(&obj)
                            .cloned()
                            .unwrap_or_default(),
                    );
                    SoundChannel {
                        name: sound_path.display().to_string(),
                        notes,
                    }
                })
                .collect();
            let mine_channels = mine_map
                .into_iter()
                .map(|(obj, notes)| {
                    let sound_path = path_root.join(
                        value
                            .header
                            .wav_files
                            .get(&obj)
                            .cloned()
                            .unwrap_or_default(),
                    );
                    MineChannel {
                        name: sound_path.display().to_string(),
                        notes,
                    }
                })
                .collect();
            let key_channels = key_map
                .into_iter()
                .map(|(obj, notes)| {
                    let sound_path = path_root.join(
                        value
                            .header
                            .wav_files
                            .get(&obj)
                            .cloned()
                            .unwrap_or_default(),
                    );
                    KeyChannel {
                        name: sound_path.display().to_string(),
                        notes,
                    }
                })
                .collect();
            (sound_channels, mine_channels, key_channels)
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
            scroll_events: value
                .notes
                .scrolling_factor_changes()
                .values()
                .map(|scroll| {
                    Ok(ScrollEvent {
                        y: converter.get_pulses_at(scroll.time),
                        rate: FinF64::new(
                            scroll
                                .factor
                                .clone()
                                .try_into()
                                .map_err(|_| BmsonConvertError::InvalidScrollingFactor)?,
                        )
                        .ok_or(BmsonConvertError::InvalidScrollingFactor)?,
                    })
                })
                .collect::<Result<Vec<_>, BmsonConvertError>>()?,
            mine_channels,
            key_channels,
        })
    }
}
