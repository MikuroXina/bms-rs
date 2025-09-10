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
#![cfg(feature = "bmson")]
#![cfg_attr(docsrs, doc(cfg(feature = "bmson")))]

pub mod bms_to_bmson;
pub mod bmson_to_bms;
pub mod fin_f64;
mod json_de;
pub mod parser;
pub mod pulse;

use std::collections::HashMap;
use std::num::NonZeroU8;

use serde::{Deserialize, Deserializer, Serialize};

use crate::bms::command::LnMode;

use self::json_de::from_json as from_json_ast;
use self::parser::{Json, parse_json};
use self::{fin_f64::FinF64, pulse::PulseNumber};

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
    pub ln_type: LnMode,
}

/// Default mode hint, beatmania 7 keys.
#[must_use]
pub fn default_mode_hint() -> String {
    "beat-7k".into()
}

/// Default relative percentage, 100%.
#[must_use]
pub fn default_percentage() -> FinF64 {
    FinF64::new(100.0).unwrap_or_else(|| {
        // This should never happen as 100.0 is a valid FinF64 value
        panic!("Internal error: 100.0 is not a valid FinF64")
    })
}

/// Default resolution pulses per quarter note in 4/4 measure, 240 pulses.
#[must_use]
pub const fn default_resolution() -> u64 {
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
    pub t: Option<LnMode>,
    /// Beatoraja implementation of long note up flag.
    /// If it is true and configured at the end position of a long note, then this position will become the ending note of the long note.
    #[serde(default)]
    pub up: Option<bool>,
}

fn deserialize_x_none_if_zero<'de, D>(deserializer: D) -> Result<Option<NonZeroU8>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<u8>::deserialize(deserializer)?;
    Ok(match opt {
        Some(0) | None => None,
        Some(v) => NonZeroU8::new(v),
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

/// bmson 解析时的告警/错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BmsonWarning {
    /// JSON 语法错误的数量。
    JsonSyntaxErrorCount(usize),
    /// 根节点不是对象。
    NonObjectRoot,
    /// 缺失字段（使用默认值填充）。
    MissingField(&'static str),
    /// 反序列化失败（类型不匹配等）。
    DeserializeFailed,
}

/// `parse_bmson` 的输出。
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct BmsonOutput {
    /// 解析后的 bmson 对象。
    pub bmson: Bmson,
    /// 解析过程中的警告。
    pub warnings: Vec<BmsonWarning>,
}

/// 解析 bmson 源字符串。不使用 `serde_json`，完全基于 chumsky + serde 反序列化。
#[must_use]
pub fn parse_bmson(src: &str) -> BmsonOutput {
    let (maybe_json, errs) = parse_json(src);
    let mut warnings: Vec<BmsonWarning> = Vec::new();
    if !errs.is_empty() {
        warnings.push(BmsonWarning::JsonSyntaxErrorCount(errs.len()));
    }

    // 准备对象根
    let mut root_map: HashMap<String, Json> = match maybe_json {
        Some(Json::Object(ref m)) => m.clone(),
        _ => {
            warnings.push(BmsonWarning::NonObjectRoot);
            HashMap::new()
        }
    };

    // 顶层缺失字段默认
    if !root_map.contains_key("version") {
        warnings.push(BmsonWarning::MissingField("version"));
        root_map.insert("version".into(), Json::Str("1.0.0".into()));
    }
    if !root_map.contains_key("sound_channels") {
        warnings.push(BmsonWarning::MissingField("sound_channels"));
        root_map.insert("sound_channels".into(), Json::Array(Vec::new()));
    }

    // info 对象缺省与字段默认
    let info_json = root_map.remove("info");
    let mut info_map: HashMap<String, Json> = match info_json {
        Some(Json::Object(m)) => m,
        _ => {
            warnings.push(BmsonWarning::MissingField("info"));
            HashMap::new()
        }
    };
    if !info_map.contains_key("title") {
        warnings.push(BmsonWarning::MissingField("info.title"));
        info_map.insert("title".into(), Json::Str(String::new()));
    }
    if !info_map.contains_key("artist") {
        warnings.push(BmsonWarning::MissingField("info.artist"));
        info_map.insert("artist".into(), Json::Str(String::new()));
    }
    if !info_map.contains_key("genre") {
        warnings.push(BmsonWarning::MissingField("info.genre"));
        info_map.insert("genre".into(), Json::Str(String::new()));
    }
    if !info_map.contains_key("level") {
        warnings.push(BmsonWarning::MissingField("info.level"));
        info_map.insert("level".into(), Json::Int(0));
    }
    if !info_map.contains_key("init_bpm") {
        warnings.push(BmsonWarning::MissingField("info.init_bpm"));
        info_map.insert("init_bpm".into(), Json::Float(120.0));
    }
    root_map.insert("info".into(), Json::Object(info_map));

    // 反序列化
    let json_root = Json::Object(root_map);
    match from_json_ast::<Bmson>(&json_root) {
        Ok(bmson) => BmsonOutput { bmson, warnings },
        Err(_) => {
            warnings.push(BmsonWarning::DeserializeFailed);
            // 构造保证可反序列化的最小对象
            let mut min_root = HashMap::new();
            let mut min_info = HashMap::new();
            min_root.insert("version".into(), Json::Str("1.0.0".into()));
            min_root.insert("sound_channels".into(), Json::Array(Vec::new()));
            min_info.insert("title".into(), Json::Str(String::new()));
            min_info.insert("artist".into(), Json::Str(String::new()));
            min_info.insert("genre".into(), Json::Str(String::new()));
            min_info.insert("level".into(), Json::Int(0));
            min_info.insert("init_bpm".into(), Json::Float(120.0));
            min_root.insert("info".into(), Json::Object(min_info));
            let min_json = Json::Object(min_root);
            let bmson = match from_json_ast::<Bmson>(&min_json) {
                Ok(b) => b,
                Err(_) => {
                    // 理论上不会失败；退化到直接复用最小 JSON 再尝试一次
                    match from_json_ast::<Bmson>(&min_json) {
                        Ok(b) => b,
                        Err(_) => {
                            // 最终兜底：返回空的占位值（尽力而为，避免 unwrap）
                            Bmson {
                                version: "1.0.0".to_string(),
                                info: BmsonInfo {
                                    title: String::new(),
                                    subtitle: String::new(),
                                    artist: String::new(),
                                    subartists: Vec::new(),
                                    genre: String::new(),
                                    mode_hint: default_mode_hint(),
                                    chart_name: String::new(),
                                    level: 0,
                                    init_bpm: FinF64::try_from(120.0)
                                        .unwrap_or_else(|_| FinF64::try_from(0.0).unwrap()),
                                    judge_rank: default_percentage(),
                                    total: default_percentage(),
                                    back_image: None,
                                    eyecatch_image: None,
                                    title_image: None,
                                    banner_image: None,
                                    preview_music: None,
                                    resolution: default_resolution(),
                                    ln_type: LnMode::default(),
                                },
                                lines: None,
                                bpm_events: Vec::new(),
                                stop_events: Vec::new(),
                                sound_channels: Vec::new(),
                                bga: Bga::default(),
                                scroll_events: Vec::new(),
                                mine_channels: Vec::new(),
                                key_channels: Vec::new(),
                            }
                        }
                    }
                }
            };
            BmsonOutput { bmson, warnings }
        }
    }
}
