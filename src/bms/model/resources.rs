//! This module introduces struct [`Resources`], which manages external resource paths.

use std::path::PathBuf;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// This aggregate manages external resource paths.
pub struct Resources {
    /// The path of MIDI file, which is played as BGM while playing the score.
    pub midi_file: Option<PathBuf>,
    /// CDDA events, indexed by value. `#CDDA`
    pub cdda: Vec<u64>,
    /// Material WAV file paths. `#MATERIALSWAV`
    pub materials_wav: Vec<PathBuf>,
    /// Material BMP file paths. `#MATERIALSBMP`
    pub materials_bmp: Vec<PathBuf>,
    /// Material path definition. `#MATERIALS`
    pub materials_path: Option<PathBuf>,
}
