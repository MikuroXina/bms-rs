//! Validity checks for BMS data after parsing and/or manual edits.
//!
//! This module provides a set of structural validations that are independent of
//! the parsing process. It can be used after editing `Bms` in-memory to ensure
//! referential integrity and basic invariants required for correct playback.

use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::bms::command::{
    ObjId,
    channel::{Key, NoteKind, PlayerSide},
    time::ObjTime,
};

use super::model::{Bms, obj::Obj};

/// Missing-related validity entries.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValidityMissing {
    /// A note references an ObjId without a corresponding WAV definition.
    #[error("Missing WAV definition for note object id: {0:?}")]
    WavForNote(ObjId),
    /// A BGM references an ObjId without a corresponding WAV definition.
    #[error("Missing WAV definition for BGM object id: {0:?}")]
    WavForBgm(ObjId),
    /// A BGA change references an ObjId without a corresponding BMP/EXBMP definition.
    #[error("Missing BMP definition for BGA object id: {0:?}")]
    BmpForBga(ObjId),
    /// A BPM change references an ObjId without a corresponding #BPMxx definition.
    #[error("Missing BPM change definition for object id: {0:?}")]
    BpmChangeDef(ObjId),
    /// A STOP event references an ObjId without a corresponding #STOPxx definition.
    #[error("Missing STOP definition for object id: {0:?}")]
    StopDef(ObjId),
}

/// Invalid-related validity entries.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValidityInvalid {
    /// A visible single note is placed in section 000 (works only on LR2).
    #[error("Visible single note placed in section 000 at {time:?} (side={side:?}, key={key:?})")]
    VisibleNoteInTrackZero {
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
    LandmineOverlapsLongAtStart {
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
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidityCheckOutput {
    /// Missing-related findings.
    pub missing: Vec<ValidityMissing>,
    /// Invalid-related findings.
    pub invalid: Vec<ValidityInvalid>,
}

impl Bms {
    /// Validate the internal consistency of `Bms` after parsing or manual edits.
    ///
    /// This performs basic referential integrity checks and data invariants that
    /// are required for correct playback, separate from parse-time checks.
    pub fn check_validity(&self) -> ValidityCheckOutput {
        let mut missing = Vec::new();
        let mut invalid = Vec::new();

        // 1) Check that every used ObjId in notes has a corresponding WAV definition.
        for obj_id in self.notes.objs.keys() {
            if !self.notes.wav_files.contains_key(obj_id) {
                missing.push(ValidityMissing::WavForNote(*obj_id));
            }
        }

        // 1.2) Check BGMs reference valid WAV ids.
        for obj_ids in self.notes.bgms.values() {
            for obj_id in obj_ids {
                if !self.notes.wav_files.contains_key(obj_id) {
                    missing.push(ValidityMissing::WavForBgm(*obj_id));
                }
            }
        }

        // 1.3) Check BGAs reference valid BMP ids.
        for bga_obj in self.graphics.bga_changes().values() {
            if !self.graphics.bmp_files.contains_key(&bga_obj.id) {
                missing.push(ValidityMissing::BmpForBga(bga_obj.id));
            }
        }

        // 1.4) Check BPM change ids used in messages have corresponding #BPMxx definitions.
        for id in &self.arrangers.bpm_change_ids_used {
            if !self.scope_defines.bpm_defs.contains_key(id) {
                missing.push(ValidityMissing::BpmChangeDef(*id));
            }
        }

        // 1.5) Check STOP ids used in messages have corresponding #STOPxx definitions.
        for id in &self.arrangers.stop_ids_used {
            if !self.scope_defines.stop_defs.contains_key(id) {
                missing.push(ValidityMissing::StopDef(*id));
            }
        }

        // 3) Placement/overlap checks for notes on lanes.
        //      - Visible notes in section 000
        //      - Overlap: visible single vs single (same time, same lane)
        //      - Overlap: visible single within long interval (same lane)
        //      - Overlap: landmine vs single (same time, same lane)
        //      - Overlap: landmine within long interval -> warn once at long start
        let mut lane_to_notes: HashMap<(PlayerSide, Key), Vec<&Obj>> = HashMap::new();
        for notes in self.notes.objs.values() {
            for obj in notes {
                // Visible note in section 000 (track index 0)
                if obj.kind == NoteKind::Visible && obj.offset.track.0 == 0 {
                    invalid.push(ValidityInvalid::VisibleNoteInTrackZero {
                        side: obj.side,
                        key: obj.key,
                        time: obj.offset,
                    });
                }
                lane_to_notes
                    .entry((obj.side, obj.key))
                    .or_default()
                    .push(obj);
            }
        }
        for ((side, key), objs) in lane_to_notes.into_iter() {
            if objs.is_empty() {
                continue;
            }
            // Sort by time
            let mut lane_objs = objs;
            lane_objs.sort_unstable_by_key(|o| o.offset);

            // Build LN intervals by pairing consecutive Long notes
            let long_times: Vec<ObjTime> = lane_objs
                .iter()
                .filter(|o| o.kind == NoteKind::Long)
                .map(|o| o.offset)
                .collect();
            let mut ln_intervals: Vec<(ObjTime, ObjTime)> = Vec::new();
            let mut iter = long_times.clone().into_iter();
            while let Some(start) = iter.next() {
                if let Some(end) = iter.next() {
                    if end >= start {
                        ln_intervals.push((start, end));
                    }
                } else {
                    break;
                }
            }

            // Overlap single vs single at the same time
            let mut i = 0;
            while i < lane_objs.len() {
                let time = lane_objs[i].offset;
                let mut j = i;
                let mut visible_count = 0usize;
                while j < lane_objs.len() && lane_objs[j].offset == time {
                    if lane_objs[j].kind == NoteKind::Visible {
                        visible_count += 1;
                    }
                    j += 1;
                }
                if visible_count >= 2 {
                    invalid.push(ValidityInvalid::OverlapVisibleSingleWithSingle {
                        side,
                        key,
                        time,
                    });
                }
                i = j;
            }

            // Overlap landmine vs single at the same time
            let mut i = 0;
            while i < lane_objs.len() {
                let time = lane_objs[i].offset;
                let mut j = i;
                let mut has_visible = false;
                let mut has_landmine = false;
                while j < lane_objs.len() && lane_objs[j].offset == time {
                    match lane_objs[j].kind {
                        NoteKind::Visible => has_visible = true,
                        NoteKind::Landmine => has_landmine = true,
                        _ => {}
                    }
                    j += 1;
                }
                if has_visible && has_landmine {
                    invalid.push(ValidityInvalid::OverlapLandmineWithSingle { side, key, time });
                }
                i = j;
            }

            // Helper: check if a time is within [s, e]
            let time_overlaps_any_ln = |t: ObjTime| -> Option<(ObjTime, ObjTime)> {
                // Early return if no long notes exist
                if long_times.is_empty() {
                    return None;
                }
                // Use binary search on sorted long_times to find the insertion point for t
                let pos = long_times.partition_point(|&x| x < t);

                // Check if we're exactly at a long note time
                if pos < long_times.len() && long_times[pos] == t {
                    // We're exactly at a long note time
                    if pos % 2 == 0 && pos + 1 < long_times.len() {
                        // Even index: this is a start of an interval
                        // Check if next element is the end (handles zero-length case)
                        return Some((long_times[pos], long_times[pos + 1]));
                    } else if pos > 0 && long_times[pos - 1] == long_times[pos] {
                        // Odd index: this is an end of an interval
                        // We're at the end time, which should not be considered "inside" the interval
                        // But for zero-length intervals, start == end, so we need to check if
                        // this end matches the previous start
                        return Some((long_times[pos - 1], long_times[pos]));
                    }
                } else if pos % 2 == 1 && long_times[pos - 1] <= t {
                    // We're positioned at the end of an interval
                    // The interval we're potentially inside is [long_times[pos-1], long_times[pos]]
                    // Since we know long_times[pos] > t (from partition_point), we just need to check
                    // if long_times[pos-1] <= t
                    return Some((long_times[pos - 1], long_times[pos]));
                }

                None
            };

            // Overlap single vs long: any visible single inside any LN interval
            for o in &lane_objs {
                if o.kind == NoteKind::Visible
                    && let Some((s, e)) = time_overlaps_any_ln(o.offset)
                {
                    invalid.push(ValidityInvalid::OverlapVisibleSingleWithLong {
                        side,
                        key,
                        time: o.offset,
                        ln_start: s,
                        ln_end: e,
                    });
                }
            }

            // Landmine vs long: warn once per LN interval at the long start
            // if any landmine appears inside that interval (including at start).
            let mut warned_ln_intervals: HashSet<(ObjTime, ObjTime)> = HashSet::new();
            for o in &lane_objs {
                if o.kind == NoteKind::Landmine
                    && let Some((s, e)) = time_overlaps_any_ln(o.offset)
                    && warned_ln_intervals.insert((s, e))
                {
                    invalid.push(ValidityInvalid::LandmineOverlapsLongAtStart {
                        side,
                        key,
                        ln_start: s,
                        ln_end: e,
                    });
                }
            }
        }

        ValidityCheckOutput { missing, invalid }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bms::{
        command::{
            ObjId,
            channel::{Key, NoteKind, PlayerSide},
            time::ObjTime,
        },
        parse::model::{
            Notes,
            obj::{BgaLayer, BgaObj, Obj},
        },
    };

    fn t(track: u64, num: u64, den: u64) -> ObjTime {
        ObjTime::new(track, num, den)
    }

    #[test]
    fn test_missing_wav_for_note() {
        let mut bms = Bms::default();
        let id = ObjId::try_from("0A").unwrap();
        let time = t(1, 0, 4);
        // Insert note via push_note to keep ids_by_key consistent
        let mut notes = Notes::default();
        notes.push_note(Obj {
            offset: time,
            kind: NoteKind::Visible,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id,
        });
        bms.notes = notes;
        // No WAV defined for id
        let out = bms.check_validity();
        assert!(out.missing.contains(&ValidityMissing::WavForNote(id)));
    }

    #[test]
    fn test_missing_bmp_for_bga() {
        let mut bms = Bms::default();
        let id = ObjId::try_from("0B").unwrap();
        let time = t(1, 0, 4);
        bms.graphics.bga_changes.insert(
            time,
            BgaObj {
                time,
                id,
                layer: BgaLayer::Base,
            },
        );
        let out = bms.check_validity();
        assert!(out.missing.contains(&ValidityMissing::BmpForBga(id)));
    }

    #[test]
    fn test_visible_note_in_track_zero() {
        let mut bms = Bms::default();
        let id = ObjId::try_from("10").unwrap();
        let time = t(0, 0, 4);
        let mut notes = Notes::default();
        notes.push_note(Obj {
            offset: time,
            kind: NoteKind::Visible,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id,
        });
        bms.notes = notes;

        let out = bms.check_validity();
        assert!(out.invalid.iter().any(|e| matches!(
            e,
            ValidityInvalid::VisibleNoteInTrackZero { time: t0, side: PlayerSide::Player1, key: Key::Key1 } if *t0 == time
        )));
    }

    #[test]
    fn test_overlap_visible_single_with_single() {
        let mut bms = Bms::default();
        let id1 = ObjId::try_from("01").unwrap();
        let id2 = ObjId::try_from("02").unwrap();
        let time = t(1, 0, 4);
        let mut notes = Notes::default();
        notes.push_note(Obj {
            offset: time,
            kind: NoteKind::Visible,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id1,
        });
        notes.push_note(Obj {
            offset: time,
            kind: NoteKind::Visible,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id2,
        });
        bms.notes = notes;

        let out = bms.check_validity();
        assert!(out.invalid.iter().any(|e| matches!(
            e,
            ValidityInvalid::OverlapVisibleSingleWithSingle { time: t0, side: PlayerSide::Player1, key: Key::Key1 } if *t0 == time
        )));
    }

    #[test]
    fn test_overlap_visible_single_with_long() {
        let mut bms = Bms::default();
        let id_ln_s = ObjId::try_from("0E").unwrap();
        let id_ln_e = ObjId::try_from("0F").unwrap();
        let id_vis = ObjId::try_from("03").unwrap();
        let ln_start = t(2, 0, 4);
        let ln_end = t(2, 2, 4);
        let vis_time = t(2, 1, 4);
        let mut notes = Notes::default();
        // LN start
        notes.push_note(Obj {
            offset: ln_start,
            kind: NoteKind::Long,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id_ln_s,
        });
        // LN end
        notes.push_note(Obj {
            offset: ln_end,
            kind: NoteKind::Long,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id_ln_e,
        });
        // Visible inside LN interval
        notes.push_note(Obj {
            offset: vis_time,
            kind: NoteKind::Visible,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id_vis,
        });
        bms.notes = notes;

        let out = bms.check_validity();
        assert!(out.invalid.iter().any(|e| matches!(
            e,
            ValidityInvalid::OverlapVisibleSingleWithLong { side: PlayerSide::Player1, key: Key::Key1, time: t0, ln_start: s, ln_end: e } if *t0 == vis_time && *s == ln_start && *e == ln_end
        )));
    }

    #[test]
    fn test_landmine_overlap_long_warn_at_start() {
        let mut bms = Bms::default();
        let id_ln_s = ObjId::try_from("1A").unwrap();
        let id_ln_e = ObjId::try_from("1B").unwrap();
        let id_mine = ObjId::try_from("1C").unwrap();
        let ln_start = t(3, 0, 4);
        let ln_end = t(3, 2, 4);
        let mine_time = t(3, 0, 4);
        let mut notes = Notes::default();
        // LN interval
        notes.push_note(Obj {
            offset: ln_start,
            kind: NoteKind::Long,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id_ln_s,
        });
        notes.push_note(Obj {
            offset: ln_end,
            kind: NoteKind::Long,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id_ln_e,
        });
        // Landmine inside the LN
        notes.push_note(Obj {
            offset: mine_time,
            kind: NoteKind::Landmine,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id_mine,
        });
        bms.notes = notes;

        let out = bms.check_validity();
        assert!(out.invalid.iter().any(|e| matches!(
            e,
            ValidityInvalid::LandmineOverlapsLongAtStart { side: PlayerSide::Player1, key: Key::Key1, ln_start: s, ln_end: e } if *s == ln_start && *e == ln_end
        )));
    }

    #[test]
    fn test_overlap_landmine_with_single() {
        let mut bms = Bms::default();
        let id_vis = ObjId::try_from("04").unwrap();
        let id_mine = ObjId::try_from("05").unwrap();
        let time = t(1, 0, 4);
        let mut notes = Notes::default();
        notes.push_note(Obj {
            offset: time,
            kind: NoteKind::Visible,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id_vis,
        });
        notes.push_note(Obj {
            offset: time,
            kind: NoteKind::Landmine,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id_mine,
        });
        bms.notes = notes;

        let out = bms.check_validity();
        assert!(out.invalid.iter().any(|e| matches!(
            e,
            ValidityInvalid::OverlapLandmineWithSingle { time: t0, side: PlayerSide::Player1, key: Key::Key1 } if *t0 == time
        )));
    }

    #[test]
    fn test_zero_length_long_note_overlap() {
        let mut bms = Bms::default();
        let id_ln_start = ObjId::try_from("20").unwrap();
        let id_ln_end = ObjId::try_from("21").unwrap();
        let id_vis = ObjId::try_from("22").unwrap();
        let zero_length_time = t(2, 0, 4);
        let vis_time = t(2, 0, 4); // Same time as zero-length LN
        let mut notes = Notes::default();

        // Zero-length long note: start and end at same time
        notes.push_note(Obj {
            offset: zero_length_time,
            kind: NoteKind::Long,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id_ln_start,
        });
        notes.push_note(Obj {
            offset: zero_length_time, // Same time - zero length
            kind: NoteKind::Long,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id_ln_end,
        });

        // Visible note at the same time as zero-length LN
        notes.push_note(Obj {
            offset: vis_time,
            kind: NoteKind::Visible,
            side: PlayerSide::Player1,
            key: Key::Key1,
            obj: id_vis,
        });

        bms.notes = notes;

        let out = bms.check_validity();
        // This should detect the overlap, but currently fails due to the bug
        assert!(
            out.invalid.iter().any(|e| matches!(
                e,
                ValidityInvalid::OverlapVisibleSingleWithLong {
                    side: PlayerSide::Player1,
                    key: Key::Key1,
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
