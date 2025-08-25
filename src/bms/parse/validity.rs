//! Validity checks for BMS data after parsing and/or manual edits.
//!
//! This module provides a set of structural validations that are independent of
//! the parsing process. It can be used after editing `Bms` in-memory to ensure
//! referential integrity and basic invariants required for correct playback.

use thiserror::Error;

use crate::bms::command::{
    ObjId,
    channel::{Key, PlayerSide},
    time::ObjTime,
};

use super::model::{Bms, obj::Obj};

/// Warnings for validity check. These issues may degrade experience but are not fatal.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValidityWarning {
    /// Defined WAV object id is never referenced by any note/BGM.
    #[error("Unused WAV object id: {0:?}")]
    UnusedWavObjectId(ObjId),
    /// Defined BMP object id is never referenced by any BGA change.
    #[error("Unused BMP object id: {0:?}")]
    UnusedBmpObjectId(ObjId),
}

/// Errors for validity check. These issues likely break playback correctness.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValidityError {
    /// A note references an ObjId without a corresponding WAV definition.
    #[error("Missing WAV definition for note object id: {0:?}")]
    MissingWavForNote(ObjId),
    /// A BGM references an ObjId without a corresponding WAV definition.
    #[error("Missing WAV definition for BGM object id: {0:?}")]
    MissingWavForBgm(ObjId),
    /// A BGA change references an ObjId without a corresponding BMP/EXBMP definition.
    #[error("Missing BMP definition for BGA object id: {0:?}")]
    MissingBmpForBga(ObjId),
    /// The per-key time index (`ids_by_key`) is missing or mismatched for an existing note.
    #[error(
        "Key index mismatch for note at time {time:?} (side={side:?}, key={key:?}), expected mapping to {expected:?}"
    )]
    IdsByKeyMismatch {
        /// Player side where the note is placed.
        side: PlayerSide,
        /// Key lane where the note is placed.
        key: Key,
        /// Timestamp of the note.
        time: ObjTime,
        /// Expected object id that should be mapped at this timestamp.
        expected: ObjId,
    },
}

/// Output of validity checks.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidityCheckOutput {
    /// List of non-fatal warnings discovered by the validator.
    pub validity_warnings: Vec<ValidityWarning>,
    /// List of fatal errors discovered by the validator.
    pub validity_errors: Vec<ValidityError>,
}

impl Bms {
    /// Validate the internal consistency of `Bms` after parsing or manual edits.
    ///
    /// This performs basic referential integrity checks and data invariants that
    /// are required for correct playback, separate from parse-time checks.
    pub fn check_validity(&self) -> ValidityCheckOutput {
        let mut validity_warnings = Vec::new();
        let mut validity_errors = Vec::new();

        // 1) Check that every used ObjId in notes has a corresponding WAV definition.
        for (obj_id, notes) in &self.notes.objs {
            if !self.notes.wav_files.contains_key(obj_id) {
                validity_errors.push(ValidityError::MissingWavForNote(*obj_id));
            }

            // 2) Ensure ids_by_key contains the expected mapping for each note entry.
            for Obj {
                side,
                key,
                offset,
                obj,
                ..
            } in notes
            {
                if let Some(key_map) = self.notes.ids_by_key.get(&(*side, *key)) {
                    if let Some(mapped) = key_map.get(offset) {
                        if mapped != obj {
                            validity_errors.push(ValidityError::IdsByKeyMismatch {
                                side: *side,
                                key: *key,
                                time: *offset,
                                expected: *obj,
                            });
                        }
                    } else {
                        validity_errors.push(ValidityError::IdsByKeyMismatch {
                            side: *side,
                            key: *key,
                            time: *offset,
                            expected: *obj,
                        });
                    }
                } else {
                    validity_errors.push(ValidityError::IdsByKeyMismatch {
                        side: *side,
                        key: *key,
                        time: *offset,
                        expected: *obj,
                    });
                }
            }
        }

        // 4) Check BGMs reference valid WAV ids.
        for obj_ids in self.notes.bgms.values() {
            for obj_id in obj_ids {
                if !self.notes.wav_files.contains_key(obj_id) {
                    validity_errors.push(ValidityError::MissingWavForBgm(*obj_id));
                }
            }
        }

        // 4.2) Check BGAs reference valid BMP ids.
        for bga_obj in self.graphics.bga_changes().values() {
            if !self.graphics.bmp_files.contains_key(&bga_obj.id) {
                validity_errors.push(ValidityError::MissingBmpForBga(bga_obj.id));
            }
        }

        // 6) Unused definitions (warnings only).
        //    - WAV definitions that are never used by any note/BGM.
        if !self.notes.wav_files.is_empty() {
            use std::collections::HashSet;
            let used_wavs: HashSet<_> = self
                .notes
                .objs
                .keys()
                .copied()
                .chain(self.notes.bgms.values().flatten().copied())
                .collect();
            for defined in self.notes.wav_files.keys() {
                if !used_wavs.contains(defined) {
                    validity_warnings.push(ValidityWarning::UnusedWavObjectId(*defined));
                }
            }
        }

        //    - BMP definitions that are never used by any BGA change.
        if !self.graphics.bmp_files.is_empty() {
            use std::collections::HashSet;
            let used_bmps: HashSet<_> = self
                .graphics
                .bga_changes()
                .values()
                .map(|bga| bga.id)
                .collect();
            for defined in self.graphics.bmp_files.keys() {
                if !used_bmps.contains(defined) {
                    validity_warnings.push(ValidityWarning::UnusedBmpObjectId(*defined));
                }
            }
        }

        ValidityCheckOutput {
            validity_warnings,
            validity_errors,
        }
    }
}
