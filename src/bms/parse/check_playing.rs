//! Check for conditions that would make this chart unplayable, or heavily affect the playing experience.

use thiserror::Error;

use crate::bms::command::ObjId;
use crate::bms::command::channel::mapper::KeyLayoutMapper;

use crate::bms::model::Bms;

#[cfg(feature = "diagnostics")]
use crate::diagnostics::{SimpleSource, ToAriadne, build_report};
#[cfg(feature = "diagnostics")]
use ariadne::{Color, Report, ReportKind};

/// Simpifies the warnings for playing, which would not make this chart unplayable.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlayingWarning {
    /// The `#TOTAL` is not specified.
    #[error("The `#TOTAL` is not specified.")]
    TotalUndefined,
    /// There is no displayable notes.
    #[error("There is no displayable notes.")]
    NoDisplayableNotes,
    /// There is no playable notes.
    #[error("There is no playable notes.")]
    NoPlayableNotes,
    /// The `#BPM` is not specified. If there are other bpm changes, the first one will be used.
    /// If there are no bpm changes, there will be an [`PlayingError::BpmUndefined`].
    #[error(
        "The `#BPM` is not specified. If there are other bpm changes, the first one will be used. If there are no bpm changes, there will be an [`PlayingError::BpmUndefined`]."
    )]
    StartBpmUndefined,
}

/// Simpifies the errors for playing, which will make this chart unplayable.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlayingError {
    /// BPM value could not be parsed.
    #[error("Invalid BPM value '{raw}': {error}")]
    InvalidBpm {
        /// The raw string value that failed to parse.
        raw: String,
        /// The parsing error details.
        error: String,
    },

    /// STOP value could not be parsed.
    #[error("Invalid STOP value for #{obj_id:02}: '{raw}'")]
    InvalidStop {
        /// The object ID that failed to parse.
        obj_id: ObjId,
        /// The raw string value that failed to parse.
        raw: String,
        /// The parsing error details.
        error: String,
    },

    /// SPEED value could not be parsed.
    #[error("Invalid SPEED value for #{obj_id:02}: '{raw}'")]
    InvalidSpeed {
        /// The object ID that failed to parse.
        obj_id: ObjId,
        /// The raw string value that failed to parse.
        raw: String,
        /// The parsing error details.
        error: String,
    },

    /// SCROLL value could not be parsed.
    #[error("Invalid SCROLL value for #{obj_id:02}: '{raw}'")]
    InvalidScroll {
        /// The object ID that failed to parse.
        obj_id: ObjId,
        /// The raw string value that failed to parse.
        raw: String,
        /// The parsing error details.
        error: String,
    },

    /// SEEK value could not be parsed.
    #[error("Invalid SEEK value for #{obj_id:02}: '{raw}'")]
    InvalidSeek {
        /// The object ID that failed to parse.
        obj_id: ObjId,
        /// The raw string value that failed to parse.
        raw: String,
        /// The parsing error details.
        error: String,
    },

    /// There is no bpm defined.
    #[error("There is no bpm defined.")]
    BpmUndefined,

    /// There is no notes.
    #[error("There is no notes.")]
    NoNotes,
}

#[cfg(feature = "diagnostics")]
impl ToAriadne for PlayingWarning {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        // Playing warnings lack precise source positions; anchor at file start.
        build_report(
            src,
            ReportKind::Warning,
            0..0,
            "Playing warning",
            &self,
            Color::Yellow,
        )
    }
}

#[cfg(feature = "diagnostics")]
impl ToAriadne for PlayingError {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        // Playing errors lack precise source positions; anchor at file start.
        build_report(
            src,
            ReportKind::Error,
            0..0,
            "Playing error",
            &self,
            Color::Red,
        )
    }
}

/// Output of checking for playing warnings and errors.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use]
pub struct PlayingCheckOutput {
    /// List of [`PlayingWarning`]s.
    pub playing_warnings: Vec<PlayingWarning>,
    /// List of [`PlayingError`]s.
    pub playing_errors: Vec<PlayingError>,
}

impl Bms {
    /// Check for playing warnings and errors based on the parsed BMS data.
    pub fn check_playing<T: KeyLayoutMapper>(&self) -> PlayingCheckOutput {
        let mut playing_warnings = Vec::new();
        let mut playing_errors = Vec::new();

        // Check for TotalUndefined warning
        if self.judge.total.is_none() {
            playing_warnings.push(PlayingWarning::TotalUndefined);
        }

        // Check for BPM-related conditions
        if self.bpm.bpm.is_none() {
            if self.bpm.bpm_changes.is_empty() {
                playing_errors.push(PlayingError::BpmUndefined);
            } else {
                playing_warnings.push(PlayingWarning::StartBpmUndefined);
            }
        }

        // Check for notes
        if self.wav.notes.is_empty() {
            playing_errors.push(PlayingError::NoNotes);
        } else {
            // Check for displayable notes (Visible, Long, Landmine)
            let has_displayable = self.wav.notes.displayables::<T>().next().is_some();
            if !has_displayable {
                playing_warnings.push(PlayingWarning::NoDisplayableNotes);
            }

            // Check for playable notes (all except Invisible)
            let has_playable = self.wav.notes.playables::<T>().next().is_some();
            if !has_playable {
                playing_warnings.push(PlayingWarning::NoPlayableNotes);
            }
        }

        PlayingCheckOutput {
            playing_warnings,
            playing_errors,
        }
    }
}
