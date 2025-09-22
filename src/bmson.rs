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
pub mod parse;
pub mod prelude;
pub mod pulse;

use std::{
    borrow::Cow,
    num::{NonZeroU8, NonZeroU64},
};

use ariadne::{Color, Label, Report, ReportKind};
use chumsky::prelude::*;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{bms::command::LnMode, diagnostics::ToAriadne};

use self::{fin_f64::FinF64, parse::parser, pulse::PulseNumber};

/// Top-level object for bmson format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bmson<'a> {
    /// Version of bmson format, which should be compared using [Semantic Version 2.0.0](http://semver.org/spec/v2.0.0.html). Older bmson file may not have this field, but lacking this must be an error.
    pub version: Cow<'a, str>,
    /// Score metadata.
    pub info: BmsonInfo<'a>,
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
    pub sound_channels: Vec<SoundChannel<'a>>,
    /// BGA data.
    #[serde(default)]
    pub bga: Bga<'a>,
    /// Beatoraja implementation of scroll events.
    #[serde(default)]
    pub scroll_events: Vec<ScrollEvent>,
    /// Beatoraja implementation of mine channel.
    #[serde(default)]
    pub mine_channels: Vec<MineChannel<'a>>,
    /// Beatoraja implementation of invisible key channel.
    #[serde(default)]
    pub key_channels: Vec<KeyChannel<'a>>,
}

/// Header metadata of chart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BmsonInfo<'a> {
    /// Self explanatory title.
    pub title: Cow<'a, str>,
    /// Self explanatory subtitle. Usually this is shown as a smaller text than `title`.
    #[serde(default)]
    pub subtitle: Cow<'a, str>,
    /// Author of the chart. It may multiple names such as `Alice vs Bob`, `Alice feat. Bob` and so on. But you should respect the value because it usually have special meaning.
    pub artist: Cow<'a, str>,
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
    pub subartists: Vec<Cow<'a, str>>,
    /// Self explanatory genre.
    pub genre: Cow<'a, str>,
    /// Hint for layout lanes, e.g. "beat-7k", "popn-5k", "generic-nkeys". Defaults to `"beat-7k"`.
    ///
    /// If you want to support many lane modes of BMS, you should check this to determine the layout for lanes. Also you can check all lane information in `sound_channels` for strict implementation.
    #[serde(default = "default_mode_hint_cow")]
    pub mode_hint: Cow<'a, str>,
    /// Special chart name, e.g. "BEGINNER", "NORMAL", "HYPER", "FOUR DIMENSIONS".
    #[serde(default)]
    pub chart_name: Cow<'a, str>,
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
    pub back_image: Option<Cow<'a, str>>,
    /// Eyecatch image file name. This should be displayed during the chart is loading.
    #[serde(default)]
    pub eyecatch_image: Option<Cow<'a, str>>,
    /// Title image file name. This should be displayed before the game starts instead of title of the music.
    #[serde(default)]
    pub title_image: Option<Cow<'a, str>>,
    /// Banner image file name. This should be displayed in music select or result scene. The aspect ratio of image is usually 15:4.
    #[serde(default)]
    pub banner_image: Option<Cow<'a, str>>,
    /// Preview music file name. This should be played when this chart is selected in a music select scene.
    #[serde(default)]
    pub preview_music: Option<Cow<'a, str>>,
    /// Numbers of pulse per quarter note in 4/4 measure. You must check this because it affects the actual seconds of `PulseNumber`.
    #[serde(
        default = "default_resolution_nonzero",
        deserialize_with = "deserialize_resolution"
    )]
    pub resolution: NonZeroU64,
    /// Beatoraja implementation of long note type.
    #[serde(default)]
    pub ln_type: LnMode,
}

/// Default mode hint, beatmania 7 keys.
#[must_use]
pub fn default_mode_hint() -> &'static str {
    "beat-7k"
}

fn default_mode_hint_cow() -> Cow<'static, str> {
    Cow::Borrowed(default_mode_hint())
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

fn default_resolution_nonzero() -> NonZeroU64 {
    NonZeroU64::new(default_resolution() as u64).expect("default_resolution should be non-zero")
}

fn deserialize_resolution<'de, D>(deserializer: D) -> Result<NonZeroU64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{Error, Visitor};
    use std::fmt;

    struct ResolutionVisitor;

    impl<'de> Visitor<'de> for ResolutionVisitor {
        type Value = NonZeroU64;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a number or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(default_resolution_nonzero())
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: Error,
        {
            match v {
                0 => Ok(default_resolution_nonzero()),
                v => Ok(NonZeroU64::new(v.abs() as u64)
                    .expect("NonZeroU64::new should not fail for non-zero i64 value")),
            }
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: Error,
        {
            match v {
                0 => Ok(default_resolution_nonzero()),
                v => Ok(NonZeroU64::new(v)
                    .expect("NonZeroU64::new should not fail for non-zero u64 value")),
            }
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: Error,
        {
            // Bmson (WebIDL unsigned long): must be an unsigned integer; negative allowed via abs
            let av = v.abs();
            if !av.is_finite() {
                return Err(E::custom("Resolution must be a finite number"));
            }
            // Reject any non-integer (has fractional part)
            if av.fract() != 0.0 {
                return Err(E::custom("Resolution must be an integer (unsigned long)"));
            }
            if av == 0.0 {
                return Ok(default_resolution_nonzero());
            }
            // Now av is a positive integer value in f64
            if av > (u64::MAX as f64) {
                return Err(E::custom(format!("Resolution value too large: {}", v)));
            }
            Ok(NonZeroU64::new(av as u64).expect(
                "NonZeroU64::new should not fail for non-zero u64 value converted from f64",
            ))
        }
    }

    deserializer.deserialize_option(ResolutionVisitor)
}

/// Event of bar line of the chart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BarLine {
    /// Pulse number to place the line.
    pub y: PulseNumber,
}

/// Note sound file and positions to be placed in the chart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundChannel<'a> {
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
    pub name: Cow<'a, str>,
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
pub struct Bga<'a> {
    /// Pictures data for playing BGA.
    #[serde(default)]
    pub bga_header: Vec<BgaHeader<'a>>,
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
pub struct BgaHeader<'a> {
    /// Self explanatory ID of picture.
    pub id: BgaId,
    /// Picture file name.
    pub name: Cow<'a, str>,
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
pub struct MineChannel<'a> {
    /// Name of the mine sound file.
    pub name: Cow<'a, str>,
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
pub struct KeyChannel<'a> {
    /// Name of the key sound file.
    pub name: Cow<'a, str>,
    /// Invisible key notes.
    pub notes: Vec<KeyEvent>,
}

/// Errors that can occur during BMSON parsing.
#[derive(Debug)]
pub enum BmsonParseError<'a> {
    /// JSON parsing error from chumsky parser.
    JsonParse {
        /// The specific parsing error.
        error: Rich<'a, char>,
    },
    /// Deserialization error from serde.
    Deserialize {
        /// The serde deserialization error.
        error: serde_path_to_error::Error<serde_json::Error>,
    },
}

impl ToAriadne for serde_path_to_error::Error<serde_json::Error> {
    fn to_report<'b>(
        &self,
        src: &crate::diagnostics::SimpleSource<'b>,
    ) -> Report<'b, (String, std::ops::Range<usize>)> {
        let filename = src.name().to_string();
        Report::build(ReportKind::Error, (filename.clone(), 0..0))
            .with_message("BMSON deserialization error")
            .with_label(
                Label::new((filename, 0..0))
                    .with_message(format!("{}", self))
                    .with_color(Color::Red),
            )
            .finish()
    }
}

impl<'a> ToAriadne for BmsonParseError<'a> {
    fn to_report<'b>(
        &self,
        src: &crate::diagnostics::SimpleSource<'b>,
    ) -> Report<'b, (String, std::ops::Range<usize>)> {
        match self {
            BmsonParseError::JsonParse { error } => error.to_report(src),
            BmsonParseError::Deserialize { error } => error.to_report(src),
        }
    }
}

/// Output of parsing a BMSON file.
///
/// This struct contains the parsed BMSON data (if successful), along with any
/// parsing errors that occurred during the process.
pub struct BmsonParseOutput<'a> {
    /// The parsed BMSON data, or None if parsing failed.
    pub bmson: Option<Bmson<'a>>,
    /// Errors that occurred during parsing.
    pub errors: Vec<BmsonParseError<'a>>,
}

/// Parse a BMSON file from JSON string.
///
/// This function provides a convenient way to parse a BMSON file in one step.
/// It uses chumsky parser internally to parse JSON, then deserializes the result
/// using `serde_path_to_error` for detailed error information.
///
/// # Returns
///
/// Returns a `BmsonParseOutput` containing the parsed BMSON data (if successful),
/// or various types of parsing errors that occurred during the process.
#[must_use]
pub fn parse_bmson<'a>(json: &'a str) -> BmsonParseOutput<'a> {
    // First parse JSON using chumsky parser
    let (value, parse_errors) = parser().parse(json.trim()).into_output_errors();

    // Collect JSON parsing errors
    let mut errors: Vec<BmsonParseError<'a>> = parse_errors
        .into_iter()
        .map(|error| BmsonParseError::JsonParse { error })
        .collect();

    // Try to get a JSON value from either chumsky or serde_json
    let json_value = value.or(serde_json::from_str(json).ok());

    // Try to deserialize the JSON value into Bmson
    let bmson = json_value
        .map(|value| serde_path_to_error::deserialize(&value))
        .map_or(None, |value| match value {
            Ok(bmson) => Some(bmson),
            Err(error) => {
                errors.push(BmsonParseError::Deserialize { error });
                None
            }
        });

    BmsonParseOutput { bmson, errors }
}
