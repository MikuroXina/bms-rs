//! Bms Processor Module.

use std::{
    collections::{BTreeMap, HashMap},
    convert::TryFrom,
    path::PathBuf,
};

use itertools::Itertools;
use strict_num_extended::{FinF64, PositiveF64};

use std::collections::HashSet;

use crate::bms::command::channel::mapper::KeyLayoutMapper;
use crate::bms::command::channel::{Key, NoteKind, PlayerSide};
use crate::bms::command::string_value::StringValue;
use crate::bms::command::time::ObjTime;
use crate::bms::model::obj::WavObj;
use crate::bms::parse::check_playing::{PlayingCheckOutput, PlayingError, PlayingWarning};
use crate::bms::parse::validity::{ValidityCheckOutput, ValidityInvalid, ValidityMissing};
use crate::bms::prelude::*;
use crate::chart::event::{ChartEvent, FlowEvent, PlayheadEvent};
use crate::chart::process::{
    AllEventsIndex, BmpId, ChartEventIdGenerator, ChartResources, Process, WavId,
};
use crate::chart::{
    Chart, DEFAULT_BPM, DEFAULT_SPEED, MAX_FIN_F64, MAX_NON_NEGATIVE_F64, TimeSpan, YCoordinate,
};
use strict_num_extended::NonNegativeF64;

/// BMS format parser.
///
/// This struct serves as a namespace for BMS parsing functions.
/// It parses BMS files and returns a `Chart` containing all precomputed data.
pub struct BmsProcessor;

/// Convert STOP duration from 192nd-note units to beats (measure units).
///
/// In 4/4 time signature:
/// - 192nd-note represents 1/192 of a whole note
/// - One measure (4/4) = 4 beats = 192/48 beats
/// - Therefore: 1 unit of 192nd-note = 1/48 beat
/// - Formula: beats = `192nd_note_value` / 48
#[must_use]
fn convert_stop_duration_to_beats(duration_192nd: NonNegativeF64) -> NonNegativeF64 {
    NonNegativeF64::new(duration_192nd.as_f64() / 48.0).unwrap_or(NonNegativeF64::ZERO)
}

impl BmsProcessor {
    /// Parse BMS file and return a `Chart` containing all precomputed data.
    ///
    /// # Errors
    ///
    /// Returns [`PlayingError::InvalidBpm`] if the BPM value could not be parsed.
    pub fn parse<T: KeyLayoutMapper>(bms: &Bms) -> Result<Chart, PlayingError> {
        // === Validate all StringValue definitions ===
        let mut errors = Vec::new();

        // Validate BPM definitions
        for string_value in bms.bpm.bpm_defs.values() {
            if let Err(e) = string_value.value() {
                errors.push(PlayingError::InvalidBpm {
                    raw: string_value.raw().to_string(),
                    error: format!("{e:?}"),
                });
            }
        }

        // Validate STOP definitions
        for (obj_id, string_value) in &bms.stop.stop_defs {
            if let Err(e) = string_value.value() {
                errors.push(PlayingError::InvalidStop {
                    obj_id: *obj_id,
                    raw: string_value.raw().to_string(),
                    error: format!("{e:?}"),
                });
            }
        }

        // Validate SPEED definitions
        for (obj_id, string_value) in &bms.speed.speed_defs {
            if let Err(e) = string_value.value() {
                errors.push(PlayingError::InvalidSpeed {
                    obj_id: *obj_id,
                    raw: string_value.raw().to_string(),
                    error: format!("{e:?}"),
                });
            }
        }

        // Validate SCROLL definitions
        for (obj_id, string_value) in &bms.scroll.scroll_defs {
            if let Err(e) = string_value.value() {
                errors.push(PlayingError::InvalidScroll {
                    obj_id: *obj_id,
                    raw: string_value.raw().to_string(),
                    error: format!("{e:?}"),
                });
            }
        }

        // Validate SEEK definitions
        for (obj_id, string_value) in &bms.video.seek_defs {
            if let Err(e) = string_value.value() {
                errors.push(PlayingError::InvalidSeek {
                    obj_id: *obj_id,
                    raw: string_value.raw().to_string(),
                    error: format!("{e:?}"),
                });
            }
        }

        // If there are errors, return the first one
        if let Some(err) = errors.into_iter().next() {
            return Err(err);
        }

        // Pre-calculate Y coordinate by tracks
        let y_memo = YMemo::new(bms);

        // Initialize BPM: prefer chart initial BPM, otherwise 120
        let init_bpm = bms
            .bpm
            .bpm
            .clone()
            .unwrap_or_else(|| StringValue::from_value(DEFAULT_BPM));

        // Precompute resource maps
        let wav_files: HashMap<WavId, PathBuf> = bms
            .wav
            .wav_files
            .iter()
            .map(|(obj_id, path)| (WavId::from(obj_id.as_u16() as usize), path.clone()))
            .collect();
        let bmp_files: HashMap<BmpId, PathBuf> = bms
            .bmp
            .bmp_files
            .iter()
            .map(|(obj_id, bmp)| (BmpId::from(obj_id.as_u16() as usize), bmp.file.clone()))
            .collect();

        let all_events = AllEventsIndex::precompute_all_events::<T>(bms, &y_memo);

        // Precompute activate times
        let all_events = precompute_activate_times(bms, &all_events, &y_memo)?;

        // Get initial BPM value
        let init_bpm_value = *init_bpm
            .value()
            .as_ref()
            .map_err(|e| PlayingError::InvalidBpm {
                raw: init_bpm.raw().to_string(),
                error: format!("{e:?}"),
            })?;

        Ok(Chart::from_parts(
            ChartResources::new(wav_files, bmp_files),
            all_events,
            y_memo.flow_events().clone(),
            init_bpm_value,
            DEFAULT_SPEED,
        ))
    }

    /// Generate measure lines for BMS (generated for each track, but not exceeding other objects' Y values)
    pub fn generate_barlines_for_bms(
        bms: &Bms,
        y_memo: &YMemo,
        events_map: &mut BTreeMap<YCoordinate, Vec<PlayheadEvent>>,
        id_gen: &mut ChartEventIdGenerator,
    ) {
        // Find the maximum Y value of all events
        let Some(max_y) = events_map.last_key_value().map(|(key, _)| *key) else {
            return;
        };

        if max_y.as_f64() <= 0.0 {
            return;
        }

        // Get the track number of the last object
        let last_obj_time = bms
            .last_obj_time()
            .unwrap_or_else(|| ObjTime::start_of(0.into()));

        // Generate measure lines for each track, but not exceeding maximum Y value
        for track in 0..=last_obj_time.track().0 {
            let track = Track(track);
            let track_y = y_memo.get_section_start_y(track);

            if track_y <= max_y {
                let event = ChartEvent::BarLine;
                let evp = PlayheadEvent::new(id_gen.next_id(), track_y, event, TimeSpan::ZERO);
                events_map.entry(track_y).or_default().push(evp);
            }
        }
    }

    pub(crate) fn lane_of_channel_id<T: KeyLayoutMapper>(
        channel_id: NoteChannelId,
    ) -> Option<(PlayerSide, Key, NoteKind)> {
        let map = channel_id.try_into_map::<T>()?;
        let side = map.side();
        let key = map.key();
        let kind = map.kind();
        Some((side, key, kind))
    }

    /// Validate the internal consistency of `Bms` after parsing or manual edits.
    ///
    /// This performs basic referential integrity checks and data invariants that
    /// are required for correct playback, separate from parse-time checks.
    pub fn check_validity<T: KeyLayoutMapper>(bms: &Bms) -> ValidityCheckOutput {
        let missing = check_missing(bms);
        let invalid = check_invalid::<T>(bms);
        ValidityCheckOutput { missing, invalid }
    }

    /// Check for playing warnings and errors based on the parsed BMS data.
    pub fn check_playing<T: KeyLayoutMapper>(bms: &Bms) -> PlayingCheckOutput {
        let mut playing_warnings = Vec::new();
        let mut playing_errors = Vec::new();

        if bms.judge.total.is_none() {
            playing_warnings.push(PlayingWarning::TotalUndefined);
        }

        if bms.bpm.bpm.is_none() {
            if bms.bpm.bpm_changes.is_empty() {
                playing_errors.push(PlayingError::BpmUndefined);
            } else {
                playing_warnings.push(PlayingWarning::StartBpmUndefined);
            }
        }

        if bms.wav.notes.is_empty() {
            playing_errors.push(PlayingError::NoNotes);
        } else {
            let has_displayable = notes_displayables::<T>(&bms.wav.notes).next().is_some();
            if !has_displayable {
                playing_warnings.push(PlayingWarning::NoDisplayableNotes);
            }

            let has_playable = notes_playables::<T>(&bms.wav.notes).next().is_some();
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

fn check_invalid<T: KeyLayoutMapper>(bms: &Bms) -> Vec<ValidityInvalid> {
    let mut invalid = vec![];

    let mut lane_to_notes: HashMap<Key, Vec<&WavObj>> = HashMap::new();
    for obj in bms.wav.notes.all_notes() {
        let Some(map) = T::from_channel_id(obj.channel_id) else {
            continue;
        };
        if map.kind().is_playable() && obj.offset.track().0 == 0 {
            invalid.push(ValidityInvalid::PlayableNoteInTrackZero {
                side: map.side(),
                key: map.key(),
                time: obj.offset,
            });
        }
        lane_to_notes.entry(map.key()).or_default().push(obj);
    }
    for (key, objs) in lane_to_notes {
        if objs.is_empty() {
            continue;
        }
        let mut lane_objs = objs;
        lane_objs.sort_unstable_by_key(|o| o.offset);

        // Single pass: bucket by kind, collect long times
        let mut long_times: Vec<ObjTime> = Vec::new();
        let mut visibles: Vec<(&WavObj, PlayerSide)> = Vec::new();
        let mut landmines: Vec<(&WavObj, PlayerSide)> = Vec::new();
        for obj in &lane_objs {
            let Some(map) = T::from_channel_id(obj.channel_id) else {
                continue;
            };
            match map.kind() {
                NoteKind::Long => long_times.push(obj.offset),
                NoteKind::Visible => visibles.push((obj, map.side())),
                NoteKind::Landmine => landmines.push((obj, map.side())),
                NoteKind::Invisible => {}
            }
        }

        // Check visible single overlap with single
        let mut single_offsets = HashSet::new();
        for (obj, side) in &visibles {
            if !single_offsets.insert(obj.offset) {
                invalid.push(ValidityInvalid::OverlapVisibleSingleWithSingle {
                    side: *side,
                    key,
                    time: obj.offset,
                });
            }
        }

        // Check landmine overlap with single
        for (obj, side) in &landmines {
            if single_offsets.contains(&obj.offset) {
                invalid.push(ValidityInvalid::OverlapLandmineWithSingle {
                    side: *side,
                    key,
                    time: obj.offset,
                });
            }
        }

        // LN overlap helper
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

        // Check visible overlap with LN
        for (obj, side) in &visibles {
            if let Some((start, end)) = time_overlaps_any_ln(obj.offset) {
                invalid.push(ValidityInvalid::OverlapVisibleSingleWithLong {
                    side: *side,
                    key,
                    time: obj.offset,
                    ln_start: start,
                    ln_end: end,
                });
            }
        }

        // Check landmine overlap with LN
        let mut warned_ln_intervals: HashSet<(ObjTime, ObjTime)> = HashSet::new();
        for (obj, side) in &landmines {
            if let Some((start, end)) = time_overlaps_any_ln(obj.offset)
                && warned_ln_intervals.insert((start, end))
            {
                invalid.push(ValidityInvalid::OverlapsLandmineLongAtStart {
                    side: *side,
                    key,
                    ln_start: start,
                    ln_end: end,
                });
            }
        }
    }
    invalid
}

/// Y coordinate memoization for efficient position calculation.
///
/// This structure caches Y coordinate calculations by track, accounting for
/// section length changes and speed modifications.
#[derive(Debug)]
pub struct YMemo {
    /// Y coordinates memoization by track, which modified its length
    y_by_track: BTreeMap<Track, FinF64>,
    speed_changes: BTreeMap<ObjTime, SpeedObj>,
    zero_length_tracks: std::collections::HashSet<Track>,
    /// Flow events that affect playback speed/scroll, organized by Y coordinate
    flow_events: BTreeMap<YCoordinate, Vec<FlowEvent>>,
}

impl YMemo {
    fn new(bms: &Bms) -> Self {
        let mut y_by_track: BTreeMap<Track, FinF64> = BTreeMap::new();
        let mut last_track = 0;
        let mut y = FinF64::ZERO;
        for (&track, section_len_change) in &bms.section_len.section_len_changes {
            let passed_sections = (track.0 - last_track).saturating_sub(1);
            y = FinF64::new(y.as_f64() + passed_sections as f64).unwrap_or(MAX_FIN_F64);
            y = (y + section_len_change.length).unwrap_or(MAX_FIN_F64);
            y_by_track.insert(track, y);
            last_track = 0;
        }

        let zero_length_tracks: std::collections::HashSet<Track> = bms
            .section_len
            .section_len_changes
            .iter()
            .filter(|(_, change)| change.length.as_f64() == 0.0)
            .map(|(&track, _)| track)
            .collect();

        // Populate flow events by Y coordinate
        let get_event_y = |time: ObjTime| -> YCoordinate {
            let section_y =
                if let Some((&track_last, track_y)) = y_by_track.range(..=&time.track()).last() {
                    let passed_sections = (time.track().0 - track_last.0).saturating_sub(1);
                    FinF64::new(passed_sections as f64 + track_y.as_f64()).unwrap_or(MAX_FIN_F64)
                } else {
                    FinF64::new(time.track().0 as f64).unwrap_or(MAX_FIN_F64)
                };
            let fraction = if time.denominator().get() > 0 {
                FinF64::new(time.numerator() as f64 / time.denominator().get() as f64)
                    .unwrap_or(FinF64::ZERO)
            } else {
                FinF64::ZERO
            };
            let factor = bms
                .speed
                .speed_factor_changes
                .range(..=time)
                .last()
                .map_or_else(|| DEFAULT_SPEED, |(_, obj)| obj.factor);
            YCoordinate::new(
                NonNegativeF64::new((section_y.as_f64() + fraction.as_f64()) * factor.as_f64())
                    .unwrap_or(MAX_NON_NEGATIVE_F64),
            )
        };

        let mut flow_events: BTreeMap<YCoordinate, Vec<FlowEvent>> = BTreeMap::new();

        // BPM changes
        for change in bms.bpm.bpm_changes.values() {
            let event_y = get_event_y(change.time);
            flow_events
                .entry(event_y)
                .or_default()
                .push(FlowEvent::Bpm(change.bpm));
        }

        // Scroll changes
        for change in bms.scroll.scrolling_factor_changes.values() {
            let event_y = get_event_y(change.time);
            flow_events
                .entry(event_y)
                .or_default()
                .push(FlowEvent::Scroll(change.factor));
        }

        // Speed changes
        for change in bms.speed.speed_factor_changes.values() {
            let event_y = get_event_y(change.time);
            flow_events
                .entry(event_y)
                .or_default()
                .push(FlowEvent::Speed(change.factor));
        }

        Self {
            y_by_track,
            speed_changes: bms.speed.speed_factor_changes.clone(),
            zero_length_tracks,
            flow_events,
        }
    }

    // Finds Y coordinate at `time` efficiently
    fn get_y(&self, time: ObjTime) -> YCoordinate {
        if self.zero_length_tracks.contains(&time.track()) {
            return self.get_section_start_y(time.track());
        }

        let section_y = {
            let track = time.track();
            if let Some((&last_track, last_y)) = self.y_by_track.range(..=&track).last() {
                let passed_sections = (track.0 - last_track.0).saturating_sub(1);
                FinF64::new(passed_sections as f64 + last_y.as_f64()).unwrap_or(MAX_FIN_F64)
            } else {
                // there is no sections modified its length until
                FinF64::new(track.0 as f64).unwrap_or(MAX_FIN_F64)
            }
        };
        let fraction = if time.denominator().get() > 0 {
            FinF64::new(time.numerator() as f64 / time.denominator().get() as f64)
                .unwrap_or(FinF64::ZERO)
        } else {
            FinF64::ZERO
        };
        let factor = self
            .speed_changes
            .range(..=time)
            .last()
            .map_or_else(|| DEFAULT_SPEED, |(_, obj)| obj.factor);
        YCoordinate::new(
            NonNegativeF64::new((section_y.as_f64() + fraction.as_f64()) * factor.as_f64())
                .unwrap_or(MAX_NON_NEGATIVE_F64),
        )
    }

    // Gets the Y coordinate at the start of a track/section (without fraction)
    fn get_section_start_y(&self, track: Track) -> YCoordinate {
        let section_y = if let Some((&last_track, last_y)) = self.y_by_track.range(..=&track).last()
        {
            let passed_sections = track.0 - last_track.0;
            FinF64::new(passed_sections as f64 + last_y.as_f64()).unwrap_or(MAX_FIN_F64)
        } else {
            FinF64::new(track.0 as f64).unwrap_or(MAX_FIN_F64)
        };
        let factor = self
            .speed_changes
            .range(..=ObjTime::start_of(track))
            .last()
            .map_or_else(|| DEFAULT_SPEED, |(_, obj)| obj.factor);
        YCoordinate::new(
            NonNegativeF64::new(section_y.as_f64() * factor.as_f64())
                .unwrap_or(MAX_NON_NEGATIVE_F64),
        )
    }

    /// Get flow events organized by Y coordinate
    #[must_use]
    pub const fn flow_events(&self) -> &BTreeMap<YCoordinate, Vec<FlowEvent>> {
        &self.flow_events
    }
}

impl AllEventsIndex {
    /// Precompute all events, store grouped by Y coordinate
    /// Note: Speed effects are calculated into event positions during initialization, ensuring event trigger times remain unchanged
    #[must_use]
    pub fn precompute_all_events<T: KeyLayoutMapper>(bms: &Bms, y_memo: &YMemo) -> Self {
        let mut events_map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        let mut id_gen: ChartEventIdGenerator = ChartEventIdGenerator::default();

        let get_event_y = |time: ObjTime| -> YCoordinate { y_memo.get_y(time) };

        let note_events: Vec<(YCoordinate, WavObj)> = bms
            .notes()
            .all_notes()
            .map(|obj| (get_event_y(obj.offset), obj.clone()))
            .sorted_by(|(y1, _), (y2, _)| y1.cmp(y2))
            .collect();

        let ln_end_markers: HashSet<(ObjTime, NoteChannelId)> = note_events
            .iter()
            .filter_map(|(_, obj)| {
                obj.ln_end_for
                    .map(|end_offset| (end_offset, obj.channel_id))
            })
            .collect();

        // Use ordered Vec instead of HashMap since f64 doesn't implement Hash
        // and NonNegativeF64 doesn't implement Hash either
        let mut zero_length_key_tracker: Vec<(YCoordinate, PlayerSide, Key, usize)> = Vec::new();

        for (i, (y, obj)) in note_events.iter().enumerate() {
            let is_zero_length_section = y_memo.zero_length_tracks.contains(&obj.offset.track());
            let lane = BmsProcessor::lane_of_channel_id::<T>(obj.channel_id);

            if let Some((side, key, _)) = lane
                && is_zero_length_section
            {
                zero_length_key_tracker.push((*y, side, key, i));
            }
        }

        // Track LN start markers to prevent double-triggering (BMS format concern only).
        //
        // BMS format represents long notes using two consecutive Long note markers:
        //   - First marker: start of long note (with length calculated to next marker)
        //   - Second marker: end of long note (with no length)
        //
        // Example in BMS format:
        //   #00151:11  <- Long note start (Player1, Key1, time=1)
        //   #00351:22  <- Long note end   (Player1, Key1, time=3)
        //
        // Without this fix, both markers would trigger events, causing:
        //   1. Double-triggering: same long note fires twice (at start and end)
        //   2. Incorrect playback: end marker creates a zero-length note event
        //
        // IMPORTANT: This is purely a BMS FORMAT PARSING concern.
        // The term "started" here refers to PARSING STATE, not GAMEPLAY STATE.
        // The actual LN visibility (including LNs whose start has passed)
        // is handled by AllEventsIndex using precomputed indices.
        let mut ln_start_markers: std::collections::HashSet<(PlayerSide, Key)> =
            std::collections::HashSet::new();

        for (i, (y, obj)) in note_events.iter().enumerate() {
            if obj.ln_end_for.is_none() && ln_end_markers.contains(&(obj.offset, obj.channel_id)) {
                continue;
            }

            let is_zero_length_section = y_memo.zero_length_tracks.contains(&obj.offset.track());
            let lane = BmsProcessor::lane_of_channel_id::<T>(obj.channel_id);
            let should_include = match lane {
                Some((side, key, _)) if is_zero_length_section => zero_length_key_tracker
                    .iter()
                    .any(|(y_val, s, k, idx)| y_val == y && s == &side && k == &key && idx == &i),
                _ => true,
            };

            if should_include {
                let event = event_for_note_static::<T>(bms, y_memo, obj);

                // Fix double-triggering by skipping LN end markers.
                //
                // Logic:
                //   - When we encounter a Long note:
                //     * If its lane already has a start marker → this is the END marker, skip it
                //     * If its lane has no start marker AND it has length → this is the START marker, track it
                //     * If its lane has no start marker AND no length → edge case, ignore (no next marker)
                //
                // Result: Each long note generates exactly one event with the correct length.
                //
                // IMPORTANT: This is purely a BMS FORMAT PARSING concern.
                // The term "started" here refers to PARSING STATE, not GAMEPLAY STATE.
                // The actual LN visibility (including LNs whose start has passed)
                // is handled by AllEventsIndex using precomputed indices.
                if let ChartEvent::Note {
                    side,
                    key,
                    kind: NoteKind::Long,
                    length,
                    ..
                } = &event
                {
                    let lane_key = (*side, *key);
                    if ln_start_markers.contains(&lane_key) {
                        // This lane already has a start marker.
                        // This marker is the end of that long note.
                        // Skip it to prevent double-triggering.
                        ln_start_markers.remove(&lane_key);
                        continue;
                    }
                    if length.is_some() {
                        // This lane has no start marker.
                        // This marker is the start of a new long note.
                        // Track it so we can skip the end marker when we encounter it.
                        ln_start_markers.insert(lane_key);
                    }
                    // If length is None, this is an orphan end marker or zero-length note.
                    // Skip it silently as it doesn't represent a valid playable note.
                }

                let evp = PlayheadEvent::new(id_gen.next_id(), *y, event, TimeSpan::ZERO);
                events_map.entry(*y).or_default().push(evp);
            }
        }

        for change in bms.bpm.bpm_changes.values() {
            let y = get_event_y(change.time);
            let event = ChartEvent::BpmChange { bpm: change.bpm };
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        // Scroll change events
        for change in bms.scroll.scrolling_factor_changes.values() {
            let y = get_event_y(change.time);
            let event = ChartEvent::ScrollChange {
                factor: change.factor,
            };
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        // Speed change events
        for change in bms.speed.speed_factor_changes.values() {
            let y = get_event_y(change.time);
            let event = ChartEvent::SpeedChange {
                factor: change.factor,
            };
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        // Stop events
        for stop in bms.stop.stops.values() {
            let y = get_event_y(stop.time);
            let event = ChartEvent::Stop {
                duration: convert_stop_duration_to_beats(stop.duration),
            };
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        // BGA change events
        for bga_obj in bms.bmp.bga_changes.values() {
            let y = get_event_y(bga_obj.time);
            let bmp_index = bga_obj.id.as_u16() as usize;
            let event = ChartEvent::BgaChange {
                layer: bga_obj.layer,
                bmp_id: Some(BmpId::from(bmp_index)),
            };
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        // BGA opacity change events (requires minor-command feature)

        for (layer, opacity_changes) in &bms.bmp.bga_opacity_changes {
            for opacity_obj in opacity_changes.values() {
                let y = get_event_y(opacity_obj.time);
                let event = ChartEvent::BgaOpacityChange {
                    layer: *layer,
                    opacity: opacity_obj.opacity,
                };
                events_map.entry(y).or_default().push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y,
                    event,
                    TimeSpan::ZERO,
                ));
            }
        }

        // BGA ARGB color change events (requires minor-command feature)
        for (layer, argb_changes) in &bms.bmp.bga_argb_changes {
            for argb_obj in argb_changes.values() {
                let y = get_event_y(argb_obj.time);
                let event = ChartEvent::BgaArgbChange {
                    layer: *layer,
                    argb: argb_obj.argb,
                };
                events_map.entry(y).or_default().push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y,
                    event,
                    TimeSpan::ZERO,
                ));
            }
        }

        // BGM volume change events
        for bgm_volume_obj in bms.volume.bgm_volume_changes.values() {
            let y = get_event_y(bgm_volume_obj.time);
            let event = ChartEvent::BgmVolumeChange {
                volume: bgm_volume_obj.volume,
            };
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        // KEY volume change events
        for key_volume_obj in bms.volume.key_volume_changes.values() {
            let y = get_event_y(key_volume_obj.time);
            let event = ChartEvent::KeyVolumeChange {
                volume: key_volume_obj.volume,
            };
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        // Text display events
        for text_obj in bms.text.text_events.values() {
            let y = get_event_y(text_obj.time);
            let event = ChartEvent::TextDisplay {
                text: text_obj.text.clone(),
            };
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        // Judge level change events
        for judge_obj in bms.judge.judge_events.values() {
            let y = get_event_y(judge_obj.time);
            let event = ChartEvent::JudgeLevelChange {
                level: judge_obj.judge_level,
            };
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        for seek_obj in bms.video.seek_events.values() {
            let y = get_event_y(seek_obj.time);
            let event = ChartEvent::VideoSeek {
                seek_time: seek_obj.position.to_string().parse::<f64>().unwrap_or(0.0),
            };
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        for bga_keybound_obj in bms.bmp.bga_keybound_events.values() {
            let y = get_event_y(bga_keybound_obj.time);
            let event = ChartEvent::BgaKeybound {
                event: bga_keybound_obj.event.clone(),
            };
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        for option_obj in bms.option.option_events.values() {
            let y = get_event_y(option_obj.time);
            let event = ChartEvent::OptionChange {
                option: option_obj.option.clone(),
            };
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        BmsProcessor::generate_barlines_for_bms(bms, y_memo, &mut events_map, &mut id_gen);
        Self::new(events_map)
    }
}

/// Precompute absolute `activate_time` for all events based on BPM segmentation and Stops.
///
/// # Errors
///
/// Returns [`PlayingError::InvalidBpm`] if the initial BPM value could not be parsed.
pub fn precompute_activate_times(
    bms: &Bms,
    all_events: &AllEventsIndex,
    y_memo: &YMemo,
) -> Result<AllEventsIndex, PlayingError> {
    use itertools::Itertools;
    use std::collections::BTreeSet;

    let mut points: BTreeSet<YCoordinate> = BTreeSet::new();
    points.insert(YCoordinate::ZERO);
    points.extend(all_events.as_by_y().keys().copied());

    let init_bpm = bms
        .bpm
        .bpm
        .clone()
        .unwrap_or_else(|| StringValue::from_value(DEFAULT_BPM));

    let init_bpm_value = *init_bpm
        .value()
        .as_ref()
        .map_err(|e| PlayingError::InvalidBpm {
            raw: init_bpm.raw().to_string(),
            error: format!("{e:?}"),
        })?;

    let bpm_changes: Vec<(YCoordinate, PositiveF64)> = bms
        .bpm
        .bpm_changes
        .iter()
        .map(|(obj_time, change)| {
            let y = y_memo.get_y(*obj_time);
            (y, change.bpm)
        })
        .collect();
    points.extend(bpm_changes.iter().map(|(y, _)| *y));

    let stop_list: Vec<(YCoordinate, NonNegativeF64)> = bms
        .stop
        .stops
        .values()
        .map(|st| {
            let sy = y_memo.get_y(st.time);
            (sy, st.duration)
        })
        .sorted_by_key(|(y, _)| *y)
        .collect();

    let cum_map =
        super::calculate_cumulative_times(&points, init_bpm_value, &bpm_changes, &stop_list);

    let new_map: std::collections::BTreeMap<YCoordinate, Vec<PlayheadEvent>> = all_events
        .as_by_y()
        .iter()
        .map(|(y_coord, indices)| {
            let at_secs = cum_map.get(y_coord).copied().unwrap_or(0.0);
            let at = TimeSpan::from_duration(std::time::Duration::from_secs_f64(at_secs));
            let new_events: Vec<_> = all_events
                .as_events()
                .get(indices.clone())
                .into_iter()
                .flatten()
                .cloned()
                .map(|mut evp| {
                    evp.activate_time = at;
                    evp
                })
                .collect();
            (*y_coord, new_events)
        })
        .collect();
    Ok(AllEventsIndex::new(new_map))
}

/// Returns all the playable notes in the score for the given key layout.
///
/// # Note
/// This iterator may include dangling objects (objects with null `wav_id`) that reference
/// non-existent WAV files. These dangling objects represent invalid or unassigned notes
/// and do not affect musical playback.
/// They may originate from parsing issues in the original BMS file or from user modifications
/// to the Notes object.
///
/// To filter out dangling objects, use:
/// ```rust
/// # use bms_rs::bms::prelude::*;
/// # use bms_rs::chart::process::bms::notes_playables;
/// # use bms_rs::chart::prelude::KeyLayoutBeat;
/// # let notes = Notes::default();
/// notes_playables::<KeyLayoutBeat>(&notes).filter(|obj| !obj.wav_id.is_null())
/// # ;
/// ```
pub fn notes_playables<T: KeyLayoutMapper>(notes: &Notes) -> impl Iterator<Item = &WavObj> {
    notes.all_notes().filter(|obj| {
        obj.channel_id
            .try_into_map::<T>()
            .is_some_and(|map| map.kind().is_playable())
    })
}

/// Returns all the displayable notes in the score for the given key layout.
///
/// # Note
/// This iterator may include dangling objects (objects with null `wav_id`) that reference
/// non-existent WAV files. These dangling objects represent invalid or unassigned notes
/// and do not affect musical playback.
/// They may originate from parsing issues in the original BMS file or from user modifications
/// to the Notes object.
///
/// To filter out dangling objects, use:
/// ```rust
/// # use bms_rs::bms::prelude::*;
/// # use bms_rs::chart::process::bms::notes_displayables;
/// # use bms_rs::chart::prelude::KeyLayoutBeat;
/// # let notes = Notes::default();
/// notes_displayables::<KeyLayoutBeat>(&notes).filter(|obj| !obj.wav_id.is_null())
/// # ;
/// ```
pub fn notes_displayables<T: KeyLayoutMapper>(notes: &Notes) -> impl Iterator<Item = &WavObj> {
    notes.all_notes().filter(|obj| {
        obj.channel_id
            .try_into_map::<T>()
            .is_some_and(|map| map.kind().is_displayable())
    })
}

/// Returns all the BGM notes in the score for the given key layout.
///
/// BGM notes are defined as notes whose channel is either:
/// - The BGM channel (`01`), or
/// - Not recognized by the current `KeyLayoutMapper` (treated as BGM by default).
///
/// This means unrecognized channels from a custom `KeyLayoutMapper` will also be
/// included. If you need strict BGM-only filtering, check `NoteChannelId::bgm()` directly.
///
/// # Note
/// This iterator may include dangling objects (objects with null `wav_id`) that reference
/// non-existent WAV files. These dangling objects represent invalid or unassigned notes
/// and do not affect musical playback.
/// They may originate from parsing issues in the original BMS file or from user modifications
/// to the Notes object.
///
/// To filter out dangling objects, use:
/// ```rust
/// # use bms_rs::bms::prelude::*;
/// # use bms_rs::chart::process::bms::notes_bgms;
/// # use bms_rs::chart::prelude::KeyLayoutBeat;
/// # let notes = Notes::default();
/// notes_bgms::<KeyLayoutBeat>(&notes).filter(|obj| !obj.wav_id.is_null())
/// # ;
/// ```
pub fn notes_bgms<T: KeyLayoutMapper>(notes: &Notes) -> impl Iterator<Item = &WavObj> {
    notes.all_notes().filter(|obj| {
        obj.channel_id
            .try_into_map::<T>()
            .is_none_or(|map| !map.kind().is_displayable())
    })
}

/// Gets the time of last playable object for the given key layout.
#[must_use]
pub fn last_playable_time<T: KeyLayoutMapper>(notes: &Notes) -> Option<ObjTime> {
    notes
        .notes_in(..)
        .rev()
        .find(|(_, obj)| {
            obj.channel_id
                .try_into_map::<T>()
                .is_some_and(|map| map.kind().is_displayable())
        })
        .map(|(_, obj)| obj.offset)
}

/// Gets the time of last BGM object for the given key layout.
///
/// You can't use this to find the length of music. Because this doesn't consider that the length of sound. And visible notes may ring after all BGMs.
#[must_use]
pub fn last_bgm_time<T: KeyLayoutMapper>(notes: &Notes) -> Option<ObjTime> {
    notes
        .notes_in(..)
        .rev()
        .find(|(_, obj)| {
            obj.channel_id
                .try_into_map::<T>()
                .is_none_or(|map| !map.kind().is_displayable())
        })
        .map(|(_, obj)| obj.offset)
}

/// Generate a static chart event for a BMS note object.
///
/// This function converts a BMS `WavObj` into a `ChartEvent` with all necessary
/// information, including note type, lane assignment, and long note duration.
///
/// # Type Parameters
/// - `T`: Key layout mapper (e.g., `Beat5`, `Beat7`, `Beat10`)
///
/// # Parameters
/// - `bms`: The parsed BMS chart data
/// - `y_memo`: Y coordinate memoization for position calculation
/// - `obj`: The note object to convert
///
/// # Returns
/// - `ChartEvent::Note` for playable notes
/// - `ChartEvent::Bgm` for BGM/background audio
#[must_use]
pub fn event_for_note_static<T: KeyLayoutMapper>(
    bms: &Bms,
    y_memo: &YMemo,
    obj: &WavObj,
) -> ChartEvent {
    let y = y_memo.get_y(obj.offset);
    let wav_id = Some(WavId::from(obj.wav_id.as_u16() as usize));

    if let Some(end_offset) = obj.ln_end_for {
        let lane = match T::from_channel_id(obj.channel_id) {
            Some(l) => l,
            None => return ChartEvent::Bgm { wav_id },
        };
        let side = lane.side();
        let key = lane.key();
        let end_y = y_memo.get_y(end_offset);
        let length = NonNegativeF64::new((end_y - y).as_f64()).unwrap_or(NonNegativeF64::ZERO);
        return ChartEvent::Note {
            side,
            key,
            kind: NoteKind::Long,
            wav_id,
            length: Some(length),
            continue_play: None,
        };
    }

    let lane = BmsProcessor::lane_of_channel_id::<T>(obj.channel_id);
    let Some((side, key, kind)) = lane else {
        return ChartEvent::Bgm { wav_id };
    };
    // TODO: After all WavObj construction paths populate `ln_end_for`, this fallback
    // path (looking up the next note in the same channel) can be removed.
    let length = (kind == NoteKind::Long)
        .then(|| {
            bms.notes()
                .next_obj_by_key(obj.channel_id, obj.offset)
                .map(|next_obj| {
                    let next_y = y_memo.get_y(next_obj.offset);
                    NonNegativeF64::new((next_y - y).as_f64()).unwrap_or(NonNegativeF64::ZERO)
                })
        })
        .flatten();
    ChartEvent::Note {
        side,
        key,
        kind,
        wav_id,
        length,
        continue_play: None,
    }
}

impl TryFrom<Bms> for Chart {
    type Error = PlayingError;

    fn try_from(bms: Bms) -> Result<Self, Self::Error> {
        BmsProcessor::parse::<crate::bms::command::channel::mapper::KeyLayoutBeat>(&bms)
    }
}

impl Process for Bms {
    type Error = PlayingError;

    fn process(self) -> Result<Chart, Self::Error> {
        BmsProcessor::parse::<crate::bms::command::channel::mapper::KeyLayoutBeat>(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bms::command::channel::mapper::KeyLayoutBeat;
    use crate::bms::command::string_value::StringValue;

    /// Test that parsing fails when BPM value is invalid (non-numeric string)
    #[test]
    fn test_parse_invalid_bpm() {
        // Create a BMS object with an invalid BPM value
        let mut bms = Bms::default();
        bms.bpm.bpm = Some(StringValue::new("invalid_bpm"));

        // Try to parse, should return InvalidBpm error
        let result = BmsProcessor::parse::<KeyLayoutBeat>(&bms);

        assert!(result.is_err());
        match result {
            Err(PlayingError::InvalidBpm { raw, error }) => {
                assert_eq!(raw, "invalid_bpm");
                // Verify error message contains details
                assert!(error.contains("invalid") || error.contains("digit") || !error.is_empty());
            }
            _ => panic!("Expected PlayingError::InvalidBpm, got: {result:?}"),
        }
    }

    /// Test that parsing fails when BPM value is an empty string
    #[test]
    fn test_parse_empty_bpm() {
        let mut bms = Bms::default();
        bms.bpm.bpm = Some(StringValue::new(""));

        let result = BmsProcessor::parse::<KeyLayoutBeat>(&bms);

        assert!(result.is_err());
        match result {
            Err(PlayingError::InvalidBpm { raw, .. }) => {
                assert_eq!(raw, "");
            }
            _ => panic!("Expected PlayingError::InvalidBpm for empty BPM, got: {result:?}"),
        }
    }

    /// Test that parsing fails when BPM value is NaN-like
    #[test]
    fn test_parse_nan_bpm() {
        let mut bms = Bms::default();
        bms.bpm.bpm = Some(StringValue::new("NaN"));

        let result = BmsProcessor::parse::<KeyLayoutBeat>(&bms);

        assert!(result.is_err());
        match result {
            Err(PlayingError::InvalidBpm { raw, .. }) => {
                assert_eq!(raw, "NaN");
            }
            _ => panic!("Expected PlayingError::InvalidBpm for NaN BPM, got: {result:?}"),
        }
    }

    /// Test that parsing succeeds with default BPM (120) when no BPM is defined
    #[test]
    fn test_parse_missing_bpm_uses_default() {
        // Create a BMS object without BPM definition
        let bms = Bms::default();

        // Parse should succeed with default BPM (120)
        let result = BmsProcessor::parse::<KeyLayoutBeat>(&bms);

        assert!(
            result.is_ok(),
            "Parse should succeed with missing BPM: {result:?}"
        );
        let chart = result.unwrap();
        assert_eq!(chart.init_bpm, DEFAULT_BPM, "Should use default BPM of 120");
    }

    /// Test that parsing succeeds with valid BPM value
    #[test]
    fn test_parse_valid_bpm() {
        const TEST_BPM_150_5: PositiveF64 = PositiveF64::new_const(150.5);

        let mut bms = Bms::default();
        bms.bpm.bpm = Some(StringValue::new("150.5"));

        let result = BmsProcessor::parse::<KeyLayoutBeat>(&bms);

        assert!(
            result.is_ok(),
            "Parse should succeed with valid BPM: {result:?}"
        );
        let chart = result.unwrap();
        assert_eq!(chart.init_bpm, TEST_BPM_150_5);
    }

    /// Test that parsing succeeds with BPM value containing special characters
    #[test]
    fn test_parse_bpm_with_special_chars() {
        let mut bms = Bms::default();
        bms.bpm.bpm = Some(StringValue::new("abc123!@#"));

        let result = BmsProcessor::parse::<KeyLayoutBeat>(&bms);

        assert!(result.is_err());
        match result {
            Err(PlayingError::InvalidBpm { raw, .. }) => {
                assert_eq!(raw, "abc123!@#");
            }
            _ => {
                panic!("Expected PlayingError::InvalidBpm for special characters, got: {result:?}")
            }
        }
    }

    /// Test that error information is preserved correctly
    #[test]
    fn test_error_contains_raw_value() {
        let invalid_value = "not_a_number";
        let mut bms = Bms::default();
        bms.bpm.bpm = Some(StringValue::new(invalid_value));

        let result = BmsProcessor::parse::<KeyLayoutBeat>(&bms);

        match result {
            Err(PlayingError::InvalidBpm { raw, .. }) => {
                assert_eq!(raw, invalid_value);
            }
            _ => panic!("Expected PlayingError::InvalidBpm"),
        }
    }
}
