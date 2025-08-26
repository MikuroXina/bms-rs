//! Validity checks for BMS data after parsing and/or manual edits.
//!
//! This module provides a set of structural validations that are independent of
//! the parsing process. It can be used after editing `Bms` in-memory to ensure
//! referential integrity and basic invariants required for correct playback.

use thiserror::Error;

use crate::bms::command::{
    ObjId,
    channel::{Key, PlayerSide},
    time::{ObjTime, Track},
};

use super::model::{Bms, obj::Obj};

// Helper: collect referenced ObjIds from a definition map by comparing values.
fn referenced_ids_by_value<'a, T, K: PartialEq + 'a>(
    defs: &std::collections::HashMap<ObjId, T>,
    def_to_key: impl Fn(&T) -> &K,
    used_keys: impl IntoIterator<Item = &'a K>,
) -> std::collections::HashSet<ObjId> {
    let mut set = std::collections::HashSet::new();
    for key in used_keys {
        if let Some(id) = defs.iter().find_map(|(id, v)| {
            if def_to_key(v) == key {
                Some(*id)
            } else {
                None
            }
        }) {
            set.insert(id);
        }
    }
    set
}

// Helper: push unused definition entries for ids in `defs` not present in `used`.
fn push_unused_warnings<D, E>(
    entries: &mut Vec<E>,
    defs: &std::collections::HashMap<ObjId, D>,
    used: &std::collections::HashSet<ObjId>,
    mk_entry: impl Fn(ObjId) -> E,
) {
    for id in defs.keys() {
        if !used.contains(id) {
            entries.push(mk_entry(*id));
        }
    }
}

// Helper: push unused definition entries by an on-demand predicate over ids.
fn push_unused_by_predicate<'a, E>(
    entries: &mut Vec<E>,
    ids: impl IntoIterator<Item = &'a ObjId>,
    is_used: impl Fn(&ObjId) -> bool,
    mk_entry: impl Fn(ObjId) -> E,
) {
    for id in ids {
        if !is_used(id) {
            entries.push(mk_entry(*id));
        }
    }
}

/// Unused-related validity entries.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValidityUnused {
    /// Defined WAV object id is never referenced by any note/BGM.
    #[error("Unused WAV object id: {0:?}")]
    UnusedWavObjectId(ObjId),
    /// Defined BMP object id is never referenced by any BGA change.
    #[error("Unused BMP object id: {0:?}")]
    UnusedBmpObjectId(ObjId),
    /// Unused BPM definition id in `ScopeDefines.bpm_defs`.
    #[error("Unused BPM definition object id: {0:?}")]
    UnusedBpmDef(ObjId),
    /// Unused STOP definition id in `ScopeDefines.stop_defs`.
    #[error("Unused STOP definition object id: {0:?}")]
    UnusedStopDef(ObjId),
    /// Unused SCROLL definition id in `ScopeDefines.scroll_defs`.
    #[error("Unused SCROLL definition object id: {0:?}")]
    UnusedScrollDef(ObjId),
    /// Unused SPEED definition id in `ScopeDefines.speed_defs`.
    #[error("Unused SPEED definition object id: {0:?}")]
    UnusedSpeedDef(ObjId),
    /// Unused EXRANK definition id in `ScopeDefines.exrank_defs`.
    #[error("Unused EXRANK definition object id: {0:?}")]
    UnusedExRankDef(ObjId),
    /// Unused TEXT definition id in `Others.texts`.
    #[error("Unused TEXT definition object id: {0:?}")]
    UnusedTextDef(ObjId),
    /// Unused SEEK definition id in `Others.seek_events`.
    #[cfg(feature = "minor-command")]
    #[error("Unused SEEK definition object id: {0:?}")]
    UnusedSeekDef(ObjId),
    /// Unused CHANGE OPTION definition id in `Others.change_options`.
    #[cfg(feature = "minor-command")]
    #[error("Unused CHANGEOPTION definition object id: {0:?}")]
    UnusedChangeOptionDef(ObjId),
    /// Unused ARGB definition id in `ScopeDefines.argb_defs`.
    #[cfg(feature = "minor-command")]
    #[error("Unused ARGB definition object id: {0:?}")]
    UnusedArgbDef(ObjId),
    /// Unused SWBGA event id in `ScopeDefines.swbga_events`.
    #[cfg(feature = "minor-command")]
    #[error("Unused SWBGA event object id: {0:?}")]
    UnusedSwBgaEvent(ObjId),
}

/// Empty-related validity entries.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValidityEmpty {
    /// Text content is empty at the time.
    #[error("Empty text content at {0:?}")]
    EmptyTextAt(ObjTime),
    /// Option content is empty at the time.
    #[cfg(feature = "minor-command")]
    #[error("Empty option content at {0:?}")]
    EmptyOptionAt(ObjTime),
}

/// Missing-related validity entries.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValidityMissing {
    /// A note references an ObjId without a corresponding WAV definition.
    #[error("Missing WAV definition for note object id: {0:?}")]
    MissingWavForNote(ObjId),
    /// A BGM references an ObjId without a corresponding WAV definition.
    #[error("Missing WAV definition for BGM object id: {0:?}")]
    MissingWavForBgm(ObjId),
    /// A BGA change references an ObjId without a corresponding BMP/EXBMP definition.
    #[error("Missing BMP definition for BGA object id: {0:?}")]
    MissingBmpForBga(ObjId),
}

/// Mapping-related validity entries.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValidityMapping {
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
    /// Orphan entry found in `ids_by_key`: mapping exists but no actual note at that time.
    #[error(
        "Orphan key index mapping at time {time:?} (side={side:?}, key={key:?}), mapped to {mapped:?} but note not found"
    )]
    IdsByKeyOrphanMapping {
        /// Player side where the mapping is placed.
        side: PlayerSide,
        /// Key lane where the mapping is placed.
        key: Key,
        /// Timestamp of the mapping.
        time: ObjTime,
        /// Mapped object id.
        mapped: ObjId,
    },
}

/// Invalid-related validity entries.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValidityInvalid {
    /// Invalid BPM value (must be > 0).
    #[error("Invalid BPM value at {0:?} (must be > 0)")]
    InvalidBpmValue(ObjTime),
    /// Invalid section length value (must be > 0).
    #[error("Invalid section length for track {0:?} (must be > 0)")]
    InvalidSectionLen(Track),
    /// Invalid stop duration (must be > 0).
    #[error("Invalid stop duration at {0:?} (must be > 0)")]
    InvalidStopDuration(ObjTime),
    /// Invalid speed factor (must be > 0).
    #[error("Invalid speed factor at {0:?} (must be > 0)")]
    InvalidSpeedFactor(ObjTime),
    /// Invalid seek position (must be >= 0).
    #[cfg(feature = "minor-command")]
    #[error("Invalid seek position at {0:?} (must be >= 0)")]
    InvalidSeekPosition(ObjTime),
}

/// Output of validity checks.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidityCheckOutput {
    /// Unused-related findings.
    pub unused: Vec<ValidityUnused>,
    /// Empty-related findings.
    pub empty: Vec<ValidityEmpty>,
    /// Missing-related findings.
    pub missing: Vec<ValidityMissing>,
    /// Mapping-related findings.
    pub mapping: Vec<ValidityMapping>,
    /// Invalid-related findings.
    pub invalid: Vec<ValidityInvalid>,
}

impl Bms {
    /// Validate the internal consistency of `Bms` after parsing or manual edits.
    ///
    /// This performs basic referential integrity checks and data invariants that
    /// are required for correct playback, separate from parse-time checks.
    pub fn check_validity(&self) -> ValidityCheckOutput {
        let mut unused = Vec::new();
        let mut empty = Vec::new();
        let mut missing = Vec::new();
        let mut mapping = Vec::new();
        let mut invalid = Vec::new();

        // 1) Check that every used ObjId in notes has a corresponding WAV definition.
        for (obj_id, notes) in &self.notes.objs {
            if !self.notes.wav_files.contains_key(obj_id) {
                missing.push(ValidityMissing::MissingWavForNote(*obj_id));
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
                            mapping.push(ValidityMapping::IdsByKeyMismatch {
                                side: *side,
                                key: *key,
                                time: *offset,
                                expected: *obj,
                            });
                        }
                    } else {
                        mapping.push(ValidityMapping::IdsByKeyMismatch {
                            side: *side,
                            key: *key,
                            time: *offset,
                            expected: *obj,
                        });
                    }
                } else {
                    mapping.push(ValidityMapping::IdsByKeyMismatch {
                        side: *side,
                        key: *key,
                        time: *offset,
                        expected: *obj,
                    });
                }
            }
        }

        // 1.2) Check BGMs reference valid WAV ids.
        for obj_ids in self.notes.bgms.values() {
            for obj_id in obj_ids {
                if !self.notes.wav_files.contains_key(obj_id) {
                    missing.push(ValidityMissing::MissingWavForBgm(*obj_id));
                }
            }
        }

        // 1.3) Check BGAs reference valid BMP ids.
        for bga_obj in self.graphics.bga_changes().values() {
            if !self.graphics.bmp_files.contains_key(&bga_obj.id) {
                missing.push(ValidityMissing::MissingBmpForBga(bga_obj.id));
            }
        }

        // 2) Validate arranger objects' value ranges.
        for bpm in self.arrangers.bpm_changes.values() {
            if bpm.bpm <= crate::bms::Decimal::from(0u8) {
                invalid.push(ValidityInvalid::InvalidBpmValue(bpm.time));
            }
        }
        for sec in self.arrangers.section_len_changes.values() {
            if sec.length <= crate::bms::Decimal::from(0u8) {
                invalid.push(ValidityInvalid::InvalidSectionLen(sec.track));
            }
        }
        for stop in self.arrangers.stops.values() {
            if stop.duration <= crate::bms::Decimal::from(0u8) {
                invalid.push(ValidityInvalid::InvalidStopDuration(stop.time));
            }
        }
        // Scroll factor: no restriction other than being finite (Decimal is always finite), so no check
        for speed in self.arrangers.speed_factor_changes.values() {
            if speed.factor <= crate::bms::Decimal::from(0u8) {
                invalid.push(ValidityInvalid::InvalidSpeedFactor(speed.time));
            }
        }

        // 3) Validate graphics minor-command objects.
        // BGA opacity: u8 (0..=255) and 0 is allowed; no check

        // 4) Validate note event value ranges and content.
        // BGM/KEY volume: u8 (0..=255) and 0 is allowed; no check
        #[cfg(feature = "minor-command")]
        {
            for s in self.notes.seek_events.values() {
                if s.position < crate::bms::Decimal::from(0u8) {
                    invalid.push(ValidityInvalid::InvalidSeekPosition(s.time));
                }
            }
        }
        for t in self.notes.text_events.values() {
            if t.text.is_empty() {
                empty.push(ValidityEmpty::EmptyTextAt(t.time));
            }
        }
        #[cfg(feature = "minor-command")]
        {
            for o in self.notes.option_events.values() {
                if o.option.is_empty() {
                    empty.push(ValidityEmpty::EmptyOptionAt(o.time));
                }
            }
        }

        // 5) Check for orphan entries in ids_by_key (present mapping but no note object at that time).
        for ((side, key), time_map) in &self.notes.ids_by_key {
            for (time, mapped) in time_map {
                let no_note = self
                    .notes
                    .objs
                    .get(mapped)
                    .map(|list| list.iter().all(|n| n.offset != *time))
                    .unwrap_or(true);
                if no_note {
                    mapping.push(ValidityMapping::IdsByKeyOrphanMapping {
                        side: *side,
                        key: *key,
                        time: *time,
                        mapped: *mapped,
                    });
                }
            }
        }

        // 6) Unused definitions (warnings only).
        //    - WAV definitions that are never used by any note/BGM.
        if !self.notes.wav_files.is_empty() {
            push_unused_by_predicate(
                &mut unused,
                self.notes.wav_files.keys(),
                |id| {
                    self.notes.objs.contains_key(id)
                        || self
                            .notes
                            .bgms
                            .values()
                            .any(|vec_ids| vec_ids.iter().any(|x| x == id))
                },
                ValidityUnused::UnusedWavObjectId,
            );
        }

        //    - BMP definitions that are never used by any BGA change.
        if !self.graphics.bmp_files.is_empty() {
            push_unused_by_predicate(
                &mut unused,
                self.graphics.bmp_files.keys(),
                |id| self.graphics.bga_changes().values().any(|b| &b.id == id),
                ValidityUnused::UnusedBmpObjectId,
            );
        }

        //    - Unused defines in ScopeDefines and Others
        if !self.scope_defines.bpm_defs.is_empty() {
            let referenced = referenced_ids_by_value(
                &self.scope_defines.bpm_defs,
                |v| v,
                self.arrangers.bpm_changes.values().map(|c| &c.bpm),
            );
            push_unused_warnings(
                &mut unused,
                &self.scope_defines.bpm_defs,
                &referenced,
                ValidityUnused::UnusedBpmDef,
            );
        }
        if !self.scope_defines.stop_defs.is_empty() {
            let referenced = referenced_ids_by_value(
                &self.scope_defines.stop_defs,
                |v| v,
                self.arrangers.stops.values().map(|s| &s.duration),
            );
            push_unused_warnings(
                &mut unused,
                &self.scope_defines.stop_defs,
                &referenced,
                ValidityUnused::UnusedStopDef,
            );
        }
        if !self.scope_defines.scroll_defs.is_empty() {
            let referenced = referenced_ids_by_value(
                &self.scope_defines.scroll_defs,
                |v| v,
                self.arrangers
                    .scrolling_factor_changes
                    .values()
                    .map(|sc| &sc.factor),
            );
            push_unused_warnings(
                &mut unused,
                &self.scope_defines.scroll_defs,
                &referenced,
                ValidityUnused::UnusedScrollDef,
            );
        }
        if !self.scope_defines.speed_defs.is_empty() {
            let referenced = referenced_ids_by_value(
                &self.scope_defines.speed_defs,
                |v| v,
                self.arrangers
                    .speed_factor_changes
                    .values()
                    .map(|sp| &sp.factor),
            );
            push_unused_warnings(
                &mut unused,
                &self.scope_defines.speed_defs,
                &referenced,
                ValidityUnused::UnusedSpeedDef,
            );
        }
        if !self.scope_defines.exrank_defs.is_empty() {
            let referenced = referenced_ids_by_value(
                &self.scope_defines.exrank_defs,
                |v| &v.judge_level,
                self.notes.judge_events.values().map(|j| &j.judge_level),
            );
            push_unused_warnings(
                &mut unused,
                &self.scope_defines.exrank_defs,
                &referenced,
                ValidityUnused::UnusedExRankDef,
            );
        }
        if !self.others.texts.is_empty() {
            let referenced = referenced_ids_by_value(
                &self.others.texts,
                |v| v,
                self.notes.text_events.values().map(|t| &t.text),
            );
            push_unused_warnings(
                &mut unused,
                &self.others.texts,
                &referenced,
                ValidityUnused::UnusedTextDef,
            );
        }
        #[cfg(feature = "minor-command")]
        if !self.others.seek_events.is_empty() {
            let referenced = referenced_ids_by_value(
                &self.others.seek_events,
                |v| v,
                self.notes.seek_events.values().map(|e| &e.position),
            );
            push_unused_warnings(
                &mut unused,
                &self.others.seek_events,
                &referenced,
                ValidityUnused::UnusedSeekDef,
            );
        }
        #[cfg(feature = "minor-command")]
        if !self.others.change_options.is_empty() {
            let referenced = referenced_ids_by_value(
                &self.others.change_options,
                |v| v,
                self.notes.option_events.values().map(|e| &e.option),
            );
            push_unused_warnings(
                &mut unused,
                &self.others.change_options,
                &referenced,
                ValidityUnused::UnusedChangeOptionDef,
            );
        }
        #[cfg(feature = "minor-command")]
        if !self.scope_defines.argb_defs.is_empty() {
            let referenced = referenced_ids_by_value(
                &self.scope_defines.argb_defs,
                |v| v,
                self.graphics
                    .bga_argb_changes
                    .values()
                    .flat_map(|m| m.values())
                    .map(|a| &a.argb),
            );
            push_unused_warnings(
                &mut unused,
                &self.scope_defines.argb_defs,
                &referenced,
                ValidityUnused::UnusedArgbDef,
            );
        }
        #[cfg(feature = "minor-command")]
        if !self.scope_defines.swbga_events.is_empty() {
            let referenced = referenced_ids_by_value(
                &self.scope_defines.swbga_events,
                |v| v,
                self.notes.bga_keybound_events.values().map(|e| &e.event),
            );
            push_unused_warnings(
                &mut unused,
                &self.scope_defines.swbga_events,
                &referenced,
                ValidityUnused::UnusedSwBgaEvent,
            );
        }

        ValidityCheckOutput {
            unused,
            empty,
            missing,
            mapping,
            invalid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bms::command::{
        channel::{Key, NoteKind, PlayerSide},
        time::ObjTime,
    };
    use crate::bms::parse::model::{Notes, obj::Obj};

    fn t(track: u64, num: u64, den: u64) -> ObjTime {
        ObjTime::new(track, num, den)
    }

    #[test]
    fn test_unused_wav_bmp_detection() {
        let mut bms = Bms::default();
        // Define WAV and BMP but never used
        let wav_id = crate::bms::command::ObjId::try_from("01").unwrap();
        bms.notes.wav_files.insert(wav_id, "a.wav".into());
        let bmp_id = crate::bms::command::ObjId::try_from("02").unwrap();
        bms.graphics.bmp_files.insert(
            bmp_id,
            crate::bms::parse::model::def::Bmp {
                file: "a.bmp".into(),
                transparent_color: crate::bms::command::graphics::Argb::default(),
            },
        );

        let out = bms.check_validity();
        assert!(
            out.unused
                .contains(&ValidityUnused::UnusedWavObjectId(wav_id))
        );
        assert!(
            out.unused
                .contains(&ValidityUnused::UnusedBmpObjectId(bmp_id))
        );
        assert!(out.empty.is_empty());
        assert!(out.missing.is_empty());
        assert!(out.mapping.is_empty());
        assert!(out.invalid.is_empty());
    }

    #[test]
    fn test_missing_wav_for_note() {
        let mut bms = Bms::default();
        let id = crate::bms::command::ObjId::try_from("0A").unwrap();
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
        assert!(
            out.missing
                .contains(&ValidityMissing::MissingWavForNote(id))
        );
    }

    #[test]
    fn test_missing_bmp_for_bga() {
        let mut bms = Bms::default();
        let id = crate::bms::command::ObjId::try_from("0B").unwrap();
        let time = t(1, 0, 4);
        bms.graphics.bga_changes.insert(
            time,
            crate::bms::parse::model::obj::BgaObj {
                time,
                id,
                layer: crate::bms::parse::model::obj::BgaLayer::Base,
            },
        );
        let out = bms.check_validity();
        assert!(out.missing.contains(&ValidityMissing::MissingBmpForBga(id)));
    }

    #[test]
    fn test_ids_by_key_orphan_mapping() {
        let mut bms = Bms::default();
        let id = crate::bms::command::ObjId::try_from("0C").unwrap();
        let time = t(1, 0, 4);
        bms.notes
            .ids_by_key
            .entry((PlayerSide::Player1, Key::Key1))
            .or_default()
            .insert(time, id);
        // No corresponding note in notes.objs at this time
        let out = bms.check_validity();
        assert!(out.mapping.iter().any(
            |e| matches!(e, ValidityMapping::IdsByKeyOrphanMapping { time: t0, .. } if *t0 == time)
        ));
    }

    #[test]
    fn test_empty_text_warning_and_unused_bpm_def() {
        let mut bms = Bms::default();
        // Empty text event -> warning
        let time = t(1, 0, 4);
        bms.notes.text_events.insert(
            time,
            crate::bms::parse::model::obj::TextObj {
                time,
                text: String::new(),
            },
        );
        // Add an unused BPM def
        let bpm_id = crate::bms::command::ObjId::try_from("0D").unwrap();
        bms.scope_defines
            .bpm_defs
            .insert(bpm_id, crate::bms::Decimal::from(120u32));

        let out = bms.check_validity();
        assert!(out.empty.contains(&ValidityEmpty::EmptyTextAt(time)));
        assert!(out.unused.contains(&ValidityUnused::UnusedBpmDef(bpm_id)));
    }
}
