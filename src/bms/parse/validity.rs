//! Validity checks for BMS data after parsing and/or manual edits.
//!
//! This module provides a set of structural validations that are independent of
//! the parsing process. It can be used after editing `Bms` in-memory to ensure
//! referential integrity and basic invariants required for correct playback.

use thiserror::Error;

use crate::bms::{
    command::ObjId,
    command::channel::{Key, PlayerSide},
    command::time::ObjTime,
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
    ///
    /// Note: This variant is currently not produced by [`BmsProcessor::check_validity`](crate::chart::process::bms::BmsProcessor::check_validity).
    /// It is reserved for future use when BGM-specific WAV missing checks are implemented.
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

#[cfg(test)]
mod tests {

    use super::*;
    use crate::bms::{
        command::{
            ObjId,
            channel::{
                Key, NoteKind, PlayerSide,
                mapper::{KeyLayoutBeat, KeyLayoutMapper, KeyMapping},
            },
            time::ObjTime,
        },
        model::{
            Bms,
            notes::Notes,
            obj::{BgaLayer, BgaObj, WavObj},
        },
    };
    use crate::chart::process::bms::BmsProcessor;

    fn t(track: u64, num: u64, den: u64) -> ObjTime {
        ObjTime::new(track, num, den).expect("denominator should be non-zero")
    }

    #[test]
    fn test_missing_wav_for_note() {
        let mut bms = Bms::default();
        let id = ObjId::try_from("0A", false).unwrap();
        let time = t(1, 0, 4);
        // Insert note via push_note to keep ids_by_key consistent
        let mut notes = Notes::default();
        notes.push_note(WavObj {
            offset: time,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                .to_channel_id(),
            wav_id: id,
            ln_end_for: None,
        });
        bms.wav.notes = notes;
        // No WAV defined for id
        let out = BmsProcessor::check_validity::<KeyLayoutBeat>(&bms);
        assert!(out.missing.contains(&ValidityMissing::WavForNote(id)));
    }

    #[test]
    fn test_missing_bmp_for_bga() {
        let mut bms = Bms::default();
        let id = ObjId::try_from("0B", false).unwrap();
        let time = t(1, 0, 4);
        bms.bmp.bga_changes.insert(
            time,
            BgaObj {
                time,
                id,
                layer: BgaLayer::Base,
            },
        );
        let out = BmsProcessor::check_validity::<KeyLayoutBeat>(&bms);
        assert!(out.missing.contains(&ValidityMissing::BmpForBga(id)));
    }

    #[test]
    fn test_visible_note_in_track_zero() {
        let mut bms = Bms::default();
        let id = ObjId::try_from("10", false).unwrap();
        let time = t(0, 0, 4);
        let mut notes = Notes::default();
        notes.push_note(WavObj {
            offset: time,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                .to_channel_id(),
            wav_id: id,
            ln_end_for: None,
        });
        bms.wav.notes = notes;

        let out = BmsProcessor::check_validity::<KeyLayoutBeat>(&bms);
        assert!(out.invalid.iter().any(|e| matches!(
            e,
            ValidityInvalid::PlayableNoteInTrackZero { time: t0, side: PlayerSide::Player1, key: Key::Key(1) } if *t0 == time
        )));
    }

    #[test]
    fn test_overlap_visible_single_with_single() {
        let mut bms = Bms::default();
        let id1 = ObjId::try_from("01", false).unwrap();
        let id2 = ObjId::try_from("02", false).unwrap();
        let time = t(1, 0, 4);
        let mut notes = Notes::default();
        notes.push_note(WavObj {
            offset: time,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                .to_channel_id(),
            wav_id: id1,
            ln_end_for: None,
        });
        notes.push_note(WavObj {
            offset: time,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                .to_channel_id(),
            wav_id: id2,
            ln_end_for: None,
        });
        bms.wav.notes = notes;

        let out = BmsProcessor::check_validity::<KeyLayoutBeat>(&bms);
        assert!(out.invalid.iter().any(|e| matches!(
            e,
            ValidityInvalid::OverlapVisibleSingleWithSingle { time: t0, side: PlayerSide::Player1, key: Key::Key(1) } if *t0 == time
        )));
    }

    #[test]
    fn test_overlap_visible_single_with_long() {
        let mut bms = Bms::default();
        let id_ln_s = ObjId::try_from("0E", false).unwrap();
        let id_ln_e = ObjId::try_from("0F", false).unwrap();
        let id_vis = ObjId::try_from("03", false).unwrap();
        let ln_start = t(2, 0, 4);
        let ln_end = t(2, 2, 4);
        let vis_time = t(2, 1, 4);
        let mut notes = Notes::default();
        // LN start
        notes.push_note(WavObj {
            offset: ln_start,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Long, Key::Key(1))
                .to_channel_id(),
            wav_id: id_ln_s,
            ln_end_for: None,
        });
        // LN end
        notes.push_note(WavObj {
            offset: ln_end,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Long, Key::Key(1))
                .to_channel_id(),
            wav_id: id_ln_e,
            ln_end_for: None,
        });
        // Visible inside LN interval
        notes.push_note(WavObj {
            offset: vis_time,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                .to_channel_id(),
            wav_id: id_vis,
            ln_end_for: None,
        });
        bms.wav.notes = notes;

        let out = BmsProcessor::check_validity::<KeyLayoutBeat>(&bms);
        assert!(out.invalid.iter().any(|e| matches!(
            e,
            ValidityInvalid::OverlapVisibleSingleWithLong { side: PlayerSide::Player1, key: Key::Key(1), time: t0, ln_start: s, ln_end: e } if *t0 == vis_time && *s == ln_start && *e == ln_end
        )));
    }

    #[test]
    fn test_landmine_overlap_long_warn_at_start() {
        let mut bms = Bms::default();
        let id_ln_s = ObjId::try_from("1A", false).unwrap();
        let id_ln_e = ObjId::try_from("1B", false).unwrap();
        let id_mine = ObjId::try_from("1C", false).unwrap();
        let ln_start = t(3, 0, 4);
        let ln_end = t(3, 2, 4);
        let mine_time = t(3, 0, 4);
        let mut notes = Notes::default();
        // LN interval
        notes.push_note(WavObj {
            offset: ln_start,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Long, Key::Key(1))
                .to_channel_id(),
            wav_id: id_ln_s,
            ln_end_for: None,
        });
        notes.push_note(WavObj {
            offset: ln_end,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Long, Key::Key(1))
                .to_channel_id(),
            wav_id: id_ln_e,
            ln_end_for: None,
        });
        // Landmine inside the LN
        notes.push_note(WavObj {
            offset: mine_time,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Landmine, Key::Key(1))
                .to_channel_id(),
            wav_id: id_mine,
            ln_end_for: None,
        });
        bms.wav.notes = notes;

        let out = BmsProcessor::check_validity::<KeyLayoutBeat>(&bms);
        assert!(out.invalid.iter().any(|e| matches!(
            e,
            ValidityInvalid::OverlapsLandmineLongAtStart { side: PlayerSide::Player1, key: Key::Key(1), ln_start: s, ln_end: e } if *s == ln_start && *e == ln_end
        )));
    }

    #[test]
    fn test_overlap_landmine_with_single() {
        let mut bms = Bms::default();
        let id_vis = ObjId::try_from("04", false).unwrap();
        let id_mine = ObjId::try_from("05", false).unwrap();
        let time = t(1, 0, 4);
        let mut notes = Notes::default();
        notes.push_note(WavObj {
            offset: time,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                .to_channel_id(),
            wav_id: id_vis,
            ln_end_for: None,
        });
        notes.push_note(WavObj {
            offset: time,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Landmine, Key::Key(1))
                .to_channel_id(),
            wav_id: id_mine,
            ln_end_for: None,
        });
        bms.wav.notes = notes;

        let out = BmsProcessor::check_validity::<KeyLayoutBeat>(&bms);
        assert!(out.invalid.iter().any(|e| matches!(
            e,
            ValidityInvalid::OverlapLandmineWithSingle { time: t0, side: PlayerSide::Player1, key: Key::Key(1) } if *t0 == time
        )));
    }

    #[test]
    fn test_zero_length_long_note_overlap() {
        let mut bms = Bms::default();
        let id_ln_start = ObjId::try_from("20", false).unwrap();
        let id_ln_end = ObjId::try_from("21", false).unwrap();
        let id_vis = ObjId::try_from("22", false).unwrap();
        let zero_length_time = t(2, 0, 4);
        let vis_time = t(2, 0, 4); // Same time as zero-length LN
        let mut notes = Notes::default();

        // Zero-length long note: start and end at same time
        notes.push_note(WavObj {
            offset: zero_length_time,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Long, Key::Key(1))
                .to_channel_id(),
            wav_id: id_ln_start,
            ln_end_for: None,
        });
        notes.push_note(WavObj {
            offset: zero_length_time, // Same time - zero length
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Long, Key::Key(1))
                .to_channel_id(),
            wav_id: id_ln_end,
            ln_end_for: None,
        });

        // Visible note at the same time as zero-length LN
        notes.push_note(WavObj {
            offset: vis_time,
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                .to_channel_id(),
            wav_id: id_vis,
            ln_end_for: None,
        });

        bms.wav.notes = notes;

        let out = BmsProcessor::check_validity::<KeyLayoutBeat>(&bms);
        assert!(
            out.invalid.iter().any(|e| matches!(
                e,
                ValidityInvalid::OverlapVisibleSingleWithLong {
                    side: PlayerSide::Player1,
                    key: Key::Key(1),
                    time: t0,
                    ln_start: s,
                    ln_end: e
                } if *t0 == vis_time && *s == zero_length_time && *e == zero_length_time
            )),
            "Failed to detect overlap with zero-length long note. Current output: {:?}",
            out.invalid
        );
    }
}
