//! Definitions of the header defines.
use std::path::PathBuf;

use crate::bms::command::{
    JudgeLevel, ObjId,
    graphics::{Argb, PixelPoint, PixelSize},
};

#[cfg(feature = "minor-command")]
use crate::bms::command::minor_command::{ExWavFrequency, ExWavPan, ExWavVolume};

/// A definition for #@BGA command.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

/// A definition for #BGA command.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

/// A definition for #EXWAV command.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "minor-command")]
pub struct ExWavDef {
    /// The object ID.
    pub id: ObjId,
    /// The pan of the sound. Also called volume balance.
    /// Range: [-10000, 10000]. -10000 is leftmost, 10000 is rightmost.
    /// Default: 0.
    pub pan: ExWavPan,
    /// The volume of the sound.
    /// Range: [-10000, 0]. -10000 is 0%, 0 is 100%.
    /// Default: 0.
    pub volume: ExWavVolume,
    /// The frequency of the sound. Unit: Hz.
    /// Range: [100, 100000].
    /// Default: None.
    pub frequency: Option<ExWavFrequency>,
    /// The file path.
    pub path: PathBuf,
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

/// A definition for #EXRANK command.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExRankDef {
    /// The object ID.
    pub id: ObjId,
    /// The judge level.
    pub judge_level: JudgeLevel,
}
