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

/// Missing-related validity entries.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValidityMissing {
    /// A note references an [`ObjId`] without a corresponding `#WAV` definition.
    #[error("Missing WAV definition for note object id: {0:?}")]
    WavForNote(ObjId),
    /// A BGM references an [`ObjId`] without a corresponding `#WAV` definition.
    #[error("Missing WAV definition for BGM object id: {0:?}")]
    WavForBgm(ObjId),
    /// A BGA change references an [`ObjId`] without a corresponding `#BMP`/`#EXBMP` definition.
    #[error("Missing BMP definition for BGA object id: {0:?}")]
    BmpForBga(ObjId),
    /// A BPM change references an [`ObjId`] without a corresponding `#BPMxx` definition.
    #[error("Missing BPM change definition for object id: {0:?}")]
    BpmChangeDef(ObjId),
    /// A STOP event references an [`ObjId`] without a corresponding `#STOPxx` definition.
    #[error("Missing STOP definition for object id: {0:?}")]
    StopDef(ObjId),
}

/// Invalid-related validity entries.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValidityInvalid {
    /// A playable single note is placed in section 000 (but supported by LR2).
    #[error("Playable note placed in section 000 at {time:?} (side={side:?}, key={key:?})")]
    PlayableNoteInTrackZero {
        /// Player side where the note is placed.
        side: PlayerSide,
        /// Key lane where the note is placed.
        key: Key,
        /// Timestamp of the note.
        time: ObjTime,
    },
    /// Two or more visible single notes overlap at the same time in the same lane.
    #[error("Visible single-note overlap at {time:?} (side={side:?}, key={key:?})")]
    OverlapVisibleSingleWithSingle {
        /// Player side where the note is placed.
        side: PlayerSide,
        /// Key lane where the overlap occurs.
        key: Key,
        /// Timestamp of the overlap.
        time: ObjTime,
    },
    /// A visible single note overlaps an active long note interval in the same lane.
    #[error(
        "Visible single-note overlaps a long note at {time:?} (side={side:?}, key={key:?}; ln=[{ln_start:?}..{ln_end:?}])"
    )]
    OverlapVisibleSingleWithLong {
        /// Player side where the note is placed.
        side: PlayerSide,
        /// Key lane where the overlap occurs.
        key: Key,
        /// Timestamp of the single note.
        time: ObjTime,
        /// Start time of the long note interval.
        ln_start: ObjTime,
        /// End time of the long note interval.
        ln_end: ObjTime,
    },
    /// A landmine note overlaps a long note interval; warn only at the long start point.
    #[error(
        "Landmine overlaps a long note starting at {ln_start:?} (side={side:?}, key={key:?}; ln_end={ln_end:?})"
    )]
    OverlapsLandmineLongAtStart {
        /// Player side where the overlap occurs.
        side: PlayerSide,
        /// Key lane where the overlap occurs.
        key: Key,
        /// Start time of the long note interval.
        ln_start: ObjTime,
        /// End time of the long note interval.
        ln_end: ObjTime,
    },
    /// A landmine note overlaps a visible single note at the same time in the same lane.
    #[error("Landmine overlaps visible single note at {time:?} (side={side:?}, key={key:?})")]
    OverlapLandmineWithSingle {
        /// Player side where the overlap occurs.
        side: PlayerSide,
        /// Key lane where the overlap occurs.
        key: Key,
        /// Timestamp of the overlap.
        time: ObjTime,
    },
}

/// Output of validity checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use]
pub struct ValidityCheckOutput {
    /// Missing-related findings.
    pub missing: Vec<ValidityMissing>,
    /// Invalid-related findings.
    pub invalid: Vec<ValidityInvalid>,
}
