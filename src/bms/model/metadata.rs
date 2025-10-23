use crate::bms::prelude::*;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Metadata {
    /// The play style of the score.
    pub player: Option<PlayerMode>,
    /// The play level of the score.
    pub play_level: Option<u8>,
    /// The difficulty of the score.
    pub difficulty: Option<u8>,
    /// The email address of the author.
    pub email: Option<String>,
    /// The url of the author.
    pub url: Option<String>,
    /// The path to override the base path of the WAV file path.
    /// This allows WAV files to be referenced relative to a different directory.
    pub wav_path_root: Option<PathBuf>,
    /// Divide property. #DIVIDEPROP
    #[cfg(feature = "minor-command")]
    pub divide_prop: Option<String>,
    /// Whether the score is the octave mode.
    /// In octave mode, the chart may have different note arrangements or gameplay mechanics.
    #[cfg(feature = "minor-command")]
    pub is_octave: bool,
}
