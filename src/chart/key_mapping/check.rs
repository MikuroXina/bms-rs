//! Validity checks for BMS data, using key layout mapping.

use std::collections::{HashMap, HashSet};

use crate::bms::command::channel::NoteChannelId;
use crate::bms::command::channel::{Key, NoteKind, PlayerSide};
use crate::bms::command::time::ObjTime;
use crate::bms::model::Bms;
use crate::bms::model::obj::WavObj;
use crate::bms::parse::validity::{ValidityCheckOutput, ValidityInvalid, ValidityMissing};
use crate::chart::key_mapping::KeyLayoutBeat;
use crate::chart::key_mapping::mapper::{KeyLayoutMapper, KeyMapping};

/// Validate the internal consistency of `Bms` after parsing or manual edits.
pub fn check_bms_validity(bms: &Bms) -> ValidityCheckOutput {
    let missing = check_missing(bms);
    let invalid = check_invalid(bms);
    ValidityCheckOutput { missing, invalid }
}

fn check_missing(bms: &Bms) -> Vec<ValidityMissing> {
    let mut missing = vec![];
    for obj_id in bms.wav.notes.all_notes().map(|obj| &obj.wav_id) {
        if !bms.wav.wav_files.contains_key(obj_id) {
            missing.push(ValidityMissing::WavForNote(*obj_id));
        }
    }
    for bga_obj in bms.bmp.bga_changes.values() {
        if !bms.bmp.bmp_files.contains_key(&bga_obj.id) {
            missing.push(ValidityMissing::BmpForBga(bga_obj.id));
        }
    }
    for id in &bms.bpm.bpm_change_ids_used {
        if !bms.bpm.bpm_defs.contains_key(id) {
            missing.push(ValidityMissing::BpmChangeDef(*id));
        }
    }
    for id in &bms.stop.stop_ids_used {
        if !bms.stop.stop_defs.contains_key(id) {
            missing.push(ValidityMissing::StopDef(*id));
        }
    }
    missing
}

fn check_invalid(bms: &Bms) -> Vec<ValidityInvalid> {
    let mut invalid = vec![];
    let mut lane_to_notes: HashMap<Key, Vec<&WavObj>> = HashMap::new();
    for obj in bms.wav.notes.all_notes() {
        let Some(kind) = obj.channel_id.note_kind() else {
            continue;
        };
        let Some(side) = obj.channel_id.player_side() else {
            continue;
        };
        let key = key_from_channel(obj.channel_id);
        if kind.is_playable() && obj.offset.track().0 == 0 {
            invalid.push(ValidityInvalid::PlayableNoteInTrackZero {
                side,
                key,
                time: obj.offset,
            });
        }
        lane_to_notes.entry(key).or_default().push(obj);
    }
    for (key, objs) in lane_to_notes {
        if objs.is_empty() {
            continue;
        }
        let mut lane_objs = objs;
        lane_objs.sort_unstable_by_key(|o| o.offset);

        let long_times: Vec<ObjTime> = lane_objs
            .iter()
            .filter_map(|o| {
                let kind = o.channel_id.note_kind()?;
                (kind == NoteKind::Long).then_some(o.offset)
            })
            .collect();

        let mut single_offsets = HashSet::new();
        for single_obj in lane_objs
            .iter()
            .filter(|obj| obj.channel_id.note_kind() == Some(NoteKind::Visible))
        {
            if !single_offsets.insert(single_obj.offset) {
                let side = single_obj
                    .channel_id
                    .player_side()
                    .unwrap_or(PlayerSide::Player1);
                invalid.push(ValidityInvalid::OverlapVisibleSingleWithSingle {
                    side,
                    key,
                    time: single_obj.offset,
                });
            }
        }

        for landmine_obj in lane_objs
            .iter()
            .filter(|obj| obj.channel_id.note_kind() == Some(NoteKind::Landmine))
        {
            if single_offsets.contains(&landmine_obj.offset) {
                let side = landmine_obj
                    .channel_id
                    .player_side()
                    .unwrap_or(PlayerSide::Player1);
                invalid.push(ValidityInvalid::OverlapLandmineWithSingle {
                    side,
                    key,
                    time: landmine_obj.offset,
                });
            }
        }

        let time_overlaps_any_ln = |t: ObjTime| -> Option<(ObjTime, ObjTime)> {
            if long_times.is_empty() {
                return None;
            }
            let pos = long_times.partition_point(|&x| x < t);
            if long_times.get(pos).copied() == Some(t) {
                if pos % 2 == 0 {
                    let end = long_times.get(pos + 1).copied()?;
                    return Some((t, end));
                }
                if pos > 0 {
                    let start = long_times.get(pos - 1).copied()?;
                    if start == t {
                        return Some((start, t));
                    }
                }
                return None;
            }
            if pos % 2 == 1 {
                let start = long_times.get(pos - 1).copied()?;
                let end = long_times.get(pos).copied()?;
                if start <= t {
                    return Some((start, end));
                }
            }
            None
        };

        for single_obj in lane_objs
            .iter()
            .filter(|obj| obj.channel_id.note_kind() == Some(NoteKind::Visible))
        {
            if let Some((start, end)) = time_overlaps_any_ln(single_obj.offset) {
                let side = single_obj
                    .channel_id
                    .player_side()
                    .unwrap_or(PlayerSide::Player1);
                invalid.push(ValidityInvalid::OverlapVisibleSingleWithLong {
                    side,
                    key,
                    time: single_obj.offset,
                    ln_start: start,
                    ln_end: end,
                });
            }
        }

        let mut warned_ln_intervals: HashSet<(ObjTime, ObjTime)> = HashSet::new();
        for landmine_obj in lane_objs
            .iter()
            .filter(|obj| obj.channel_id.note_kind() == Some(NoteKind::Landmine))
        {
            if let Some((start, end)) = time_overlaps_any_ln(landmine_obj.offset)
                && warned_ln_intervals.insert((start, end))
            {
                let side = landmine_obj
                    .channel_id
                    .player_side()
                    .unwrap_or(PlayerSide::Player1);
                invalid.push(ValidityInvalid::OverlapsLandmineLongAtStart {
                    side,
                    key,
                    ln_start: start,
                    ln_end: end,
                });
            }
        }
    }
    invalid
}

fn key_from_channel(channel_id: NoteChannelId) -> Key {
    KeyLayoutBeat::from_channel_id(channel_id).map_or(Key::Key(0), |klb| klb.key())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bms::command::ObjId;
    use crate::bms::model::notes::Notes;
    use crate::bms::model::obj::{BgaLayer, BgaObj};

    fn t(track: u64, num: u64, den: u64) -> ObjTime {
        ObjTime::new(track, num, den).expect("denominator should be non-zero")
    }

    #[test]
    fn test_missing_wav_for_note() {
        let mut bms = Bms::default();
        let id = ObjId::try_from("0A", false).unwrap();
        let time = t(1, 0, 4);
        let mut notes = Notes::default();
        notes.push_note(WavObj {
            offset: time,
            channel_id: NoteChannelId::try_from([b'1', b'1']).unwrap(),
            wav_id: id,
        });
        bms.wav.notes = notes;
        let out = check_bms_validity(&bms);
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
        let out = check_bms_validity(&bms);
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
            channel_id: NoteChannelId::try_from([b'1', b'1']).unwrap(),
            wav_id: id,
        });
        bms.wav.notes = notes;

        let out = check_bms_validity(&bms);
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
            channel_id: NoteChannelId::try_from([b'1', b'1']).unwrap(),
            wav_id: id1,
        });
        notes.push_note(WavObj {
            offset: time,
            channel_id: NoteChannelId::try_from([b'1', b'1']).unwrap(),
            wav_id: id2,
        });
        bms.wav.notes = notes;

        let out = check_bms_validity(&bms);
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
        notes.push_note(WavObj {
            offset: ln_start,
            channel_id: NoteChannelId::try_from([b'5', b'1']).unwrap(),
            wav_id: id_ln_s,
        });
        notes.push_note(WavObj {
            offset: ln_end,
            channel_id: NoteChannelId::try_from([b'5', b'1']).unwrap(),
            wav_id: id_ln_e,
        });
        notes.push_note(WavObj {
            offset: vis_time,
            channel_id: NoteChannelId::try_from([b'1', b'1']).unwrap(),
            wav_id: id_vis,
        });
        bms.wav.notes = notes;

        let out = check_bms_validity(&bms);
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
        notes.push_note(WavObj {
            offset: ln_start,
            channel_id: NoteChannelId::try_from([b'5', b'1']).unwrap(),
            wav_id: id_ln_s,
        });
        notes.push_note(WavObj {
            offset: ln_end,
            channel_id: NoteChannelId::try_from([b'5', b'1']).unwrap(),
            wav_id: id_ln_e,
        });
        notes.push_note(WavObj {
            offset: mine_time,
            channel_id: NoteChannelId::try_from([b'D', b'1']).unwrap(),
            wav_id: id_mine,
        });
        bms.wav.notes = notes;

        let out = check_bms_validity(&bms);
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
            channel_id: NoteChannelId::try_from([b'1', b'1']).unwrap(),
            wav_id: id_vis,
        });
        notes.push_note(WavObj {
            offset: time,
            channel_id: NoteChannelId::try_from([b'D', b'1']).unwrap(),
            wav_id: id_mine,
        });
        bms.wav.notes = notes;

        let out = check_bms_validity(&bms);
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
        let vis_time = t(2, 0, 4);
        let mut notes = Notes::default();

        notes.push_note(WavObj {
            offset: zero_length_time,
            channel_id: NoteChannelId::try_from([b'5', b'1']).unwrap(),
            wav_id: id_ln_start,
        });
        notes.push_note(WavObj {
            offset: zero_length_time,
            channel_id: NoteChannelId::try_from([b'5', b'1']).unwrap(),
            wav_id: id_ln_end,
        });

        notes.push_note(WavObj {
            offset: vis_time,
            channel_id: NoteChannelId::try_from([b'1', b'1']).unwrap(),
            wav_id: id_vis,
        });

        bms.wav.notes = notes;

        let out = check_bms_validity(&bms);
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
