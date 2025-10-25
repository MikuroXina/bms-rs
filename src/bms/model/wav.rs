//! This module introduces struct [`WavObjects`], which manages definitions and events of sound.

use std::{collections::HashMap, path::PathBuf};

use crate::bms::{
    command::minor_command::{ExWavFrequency, ExWavPan, ExWavVolume},
    prelude::*,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// This aggregate manages definitions and events of sound.
pub struct WavObjects {
    /// The WAV file paths corresponding to the id of the note object.
    pub wav_files: HashMap<ObjId, PathBuf>,
    /// Storage of sound notes on the score.
    pub notes: Notes,
    /// Storage for `#EXWAV` definitions
    pub exwav_defs: HashMap<ObjId, ExWavDef>,
    /// WAVCMD events, indexed by `wav_index`. `#WAVCMD`
    pub wavcmd_events: HashMap<ObjId, WavCmdEvent>,
}

/// A definition for `#EXWAV` command.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
