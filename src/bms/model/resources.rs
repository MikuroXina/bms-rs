#![cfg(feature = "minor-command")]

use std::path::PathBuf;

use num::BigUint;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Resources {
    /// The path of MIDI file, which is played as BGM while playing the score.
    pub midi_file: Option<PathBuf>,
    /// CDDA events, indexed by value. `#CDDA`
    pub cdda: Vec<BigUint>,
    /// Material WAV file paths. `#MATERIALSWAV`
    pub materials_wav: Vec<PathBuf>,
    /// Material BMP file paths. `#MATERIALSBMP`
    pub materials_bmp: Vec<PathBuf>,
    /// Material path definition. `#MATERIALS`
    pub materials_path: Option<PathBuf>,
}
