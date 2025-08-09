//! Check for conditions that would make this chart unplayable, or heavily affect the playing experience.

use thiserror::Error;

use crate::bms::command::channel::NoteKind;

use super::model::Bms;

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

/// Simpifies the warnings for playing, which will make this chart unplayable.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlayingError {
    /// There is no bpm defined.
    #[error("There is no bpm defined.")]
    BpmUndefined,
    /// There is no notes.
    #[error("There is no notes.")]
    NoNotes,
}

/// Output of playing check.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlayingCheckOutput {
    /// Warnings that occurred during playing check.
    pub playing_warnings: Vec<PlayingWarning>,
    /// Errors which will make this chart unplayable.
    pub playing_errors: Vec<PlayingError>,
}

impl Bms {
    /// Check for playing warnings and errors based on the parsed BMS data.
    pub(crate) fn check_playing(&self) -> PlayingCheckOutput {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // Check for TotalUndefined warning
        if self.header.total.is_none() {
            warnings.push(PlayingWarning::TotalUndefined);
        }

        // Check for BPM-related conditions
        if self.arrangers.bpm.is_none() {
            if self.arrangers.bpm_changes.is_empty() {
                errors.push(PlayingError::BpmUndefined);
            } else {
                warnings.push(PlayingWarning::StartBpmUndefined);
            }
        }

        // Check for notes
        if self.notes.objs.is_empty() {
            errors.push(PlayingError::NoNotes);
        } else {
            // Check for displayable notes (Visible, Long, Landmine)
            let has_displayable = self.notes.all_notes().any(|note| {
                matches!(
                    note.kind,
                    NoteKind::Visible | NoteKind::Long | NoteKind::Landmine
                )
            });
            if !has_displayable {
                warnings.push(PlayingWarning::NoDisplayableNotes);
            }

            // Check for playable notes (all except Invisible)
            let has_playable = self.notes.all_notes().any(|note| note.kind.is_playable());
            if !has_playable {
                warnings.push(PlayingWarning::NoPlayableNotes);
            }
        }

        PlayingCheckOutput {
            playing_warnings: warnings,
            playing_errors: errors,
        }
    }
}
