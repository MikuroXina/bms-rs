//! Bms Processor Module.

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use itertools::Itertools;
use strict_num_extended::{FinF64, NonNegativeF64, PositiveF64};

use crate::bms::command::string_value::StringValue;
use crate::bms::parse::check_playing::PlayingError;
use crate::bms::prelude::*;
use crate::chart::event::{BmsEvent, ChartEvent, FlowEvent, PlayheadEvent};
use crate::chart::prelude::{TimeSpan, YCoordinate};
use crate::chart::process::{
    AllEventsIndex, BmpId, ChartEventIdGenerator, ChartResources, Process, StopDurationUnit, WavId,
    calculate_cumulative_times,
};
use crate::chart::{Chart, DEFAULT_BPM, DEFAULT_SPEED, MAX_FIN_F64, MAX_NON_NEGATIVE_F64};

/// BMS format parser (internal).
///
/// This struct serves as a namespace for BMS parsing functions.
/// It parses BMS files and returns a `Chart` containing all precomputed data.
///
/// Users should use [`Bms::process`](Process) via the [`Process`] trait instead.
struct BmsProcessor;

/// Convert STOP duration from 192nd-note units to beats.
///
/// `#STOPxx` value unit = 192nd note (1 value = 1/192 whole note = 1/48 beat, 4/4 time).
/// Matches beatoraja (`stop_ms = 1250 × value / bpm` ⇔ beat × 60 / bpm).
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
    pub(crate) fn generate_barlines_for_bms(
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
}

impl<L: KeyLayoutMapper> Process<L> for Bms {
    type Error = PlayingError;

    fn process(&self) -> Result<Chart, Self::Error> {
        BmsProcessor::parse::<L>(self)
    }
}

/// Y coordinate memoization for efficient position calculation.
///
/// This structure caches Y coordinate calculations by track, accounting for
/// section length changes and speed modifications.
#[derive(Debug)]
pub struct YMemo {
    /// Length offset accumulated by track that modified its length:
    /// `offset(track) = Σ_{i ≤ track}(len_i − 1.0)` (self-inclusive).
    /// measure n starts at n (index, default 1.0/measure) + `offset(previous track)`.
    y_by_track: BTreeMap<Track, FinF64>,
    /// Section length of each track that modified its length (default 1.0),
    /// used to scale the intra-track fraction (`fraction × track_len`).
    section_lengths: BTreeMap<Track, FinF64>,
    speed_changes: BTreeMap<ObjTime, SpeedObj>,
    zero_length_tracks: std::collections::HashSet<Track>,
    /// Flow events that affect playback speed/scroll, organized by Y coordinate
    flow_events: BTreeMap<YCoordinate, Vec<FlowEvent>>,
}

impl YMemo {
    fn new(bms: &Bms) -> Self {
        let mut y_by_track: BTreeMap<Track, FinF64> = BTreeMap::new();
        let mut section_lengths: BTreeMap<Track, FinF64> = BTreeMap::new();
        let mut offset = 0.0;
        for (&track, section_len_change) in &bms.section_len.section_len_changes {
            // BMS: measure n starts at n + Σ_{i<n}(len_i − 1.0).
            // Accumulate the self-inclusive offset: get_y/get_event_y query the
            // "previous" track via range(..track), so in-table tracks get an offset
            // excluding themselves, while out-of-table tracks get the last in-table one.
            offset += section_len_change.length.as_f64() - 1.0;
            y_by_track.insert(track, FinF64::new(offset).unwrap_or(MAX_FIN_F64));
            section_lengths.insert(
                track,
                FinF64::new(section_len_change.length.as_f64()).unwrap_or(MAX_FIN_F64),
            );
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
            // measure start = index + previous track's length offset (range(..track) excludes self)
            let section_y = FinF64::new(
                time.track().0 as f64
                    + y_by_track
                        .range(..time.track())
                        .last()
                        .map_or(0.0, |(_, &off)| off.as_f64()),
            )
            .unwrap_or(MAX_FIN_F64);
            let fraction = if time.denominator().get() > 0 {
                FinF64::new(time.numerator() as f64 / time.denominator().get() as f64)
                    .unwrap_or(FinF64::ZERO)
            } else {
                FinF64::ZERO
            };
            // Data spreads evenly within the track: y delta = fraction × track length (default 1.0)
            let track_len = section_lengths
                .get(&time.track())
                .map_or(1.0, |len| len.as_f64());
            let factor = bms
                .speed
                .speed_factor_changes
                .range(..=time)
                .last()
                .map_or_else(|| DEFAULT_SPEED, |(_, obj)| obj.factor);
            YCoordinate::new(
                NonNegativeF64::new(
                    (section_y.as_f64() + fraction.as_f64() * track_len) * factor.as_f64(),
                )
                .unwrap_or(MAX_NON_NEGATIVE_F64),
            )
        };

        let mut flow_events: BTreeMap<YCoordinate, Vec<FlowEvent>> = BTreeMap::new();

        // BPM changes (exBPM, channel 08)
        for change in bms.bpm.bpm_changes.values() {
            let event_y = get_event_y(change.time);
            flow_events
                .entry(event_y)
                .or_default()
                .push(FlowEvent::Bpm(change.bpm));
        }

        // BPM changes (u8, channel 03)
        // Value 0 = no event (BMS convention), skip — otherwise u8 0 would emit a
        // BPM 0 → default 120 change, polluting later sections with 120 BPM
        // ([Clue]Random's #11503 has many zero fraction slots).
        for (time, &bpm_u8) in &bms.bpm.bpm_changes_u8 {
            if bpm_u8 == 0 {
                continue;
            }
            let event_y = get_event_y(*time);
            let bpm = PositiveF64::new(bpm_u8 as f64).unwrap_or(DEFAULT_BPM);
            flow_events
                .entry(event_y)
                .or_default()
                .push(FlowEvent::Bpm(bpm));
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
            section_lengths,
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

        // measure start = index + previous track's length offset (range(..track) excludes self)
        let section_y = FinF64::new(
            time.track().0 as f64
                + self
                    .y_by_track
                    .range(..time.track())
                    .last()
                    .map_or(0.0, |(_, &off)| off.as_f64()),
        )
        .unwrap_or(MAX_FIN_F64);
        let fraction = if time.denominator().get() > 0 {
            FinF64::new(time.numerator() as f64 / time.denominator().get() as f64)
                .unwrap_or(FinF64::ZERO)
        } else {
            FinF64::ZERO
        };
        // Data spreads evenly within the track: y delta = fraction × track length (default 1.0)
        let track_len = self
            .section_lengths
            .get(&time.track())
            .map_or(1.0, |len| len.as_f64());
        let factor = self
            .speed_changes
            .range(..=time)
            .last()
            .map_or_else(|| DEFAULT_SPEED, |(_, obj)| obj.factor);
        YCoordinate::new(
            NonNegativeF64::new(
                (section_y.as_f64() + fraction.as_f64() * track_len) * factor.as_f64(),
            )
            .unwrap_or(MAX_NON_NEGATIVE_F64),
        )
    }

    // Gets the Y coordinate at the start of a track/section (without fraction)
    fn get_section_start_y(&self, track: Track) -> YCoordinate {
        let section_y = FinF64::new(
            track.0 as f64
                + self
                    .y_by_track
                    .range(..track)
                    .last()
                    .map_or(0.0, |(_, &off)| off.as_f64()),
        )
        .unwrap_or(MAX_FIN_F64);
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

        // BPM change events (u8, channel 03)
        for (time, &bpm_u8) in &bms.bpm.bpm_changes_u8 {
            let y = get_event_y(*time);
            let bpm = PositiveF64::new(bpm_u8 as f64).unwrap_or(DEFAULT_BPM);
            let event = ChartEvent::BpmChange { bpm };
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
        // `#STOPxx` value unit = 192nd note (1 value = 1/48 beat), matching `ChartEvent::Stop`.
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
                let event = ChartEvent::Bms(BmsEvent::BgaOpacityChange {
                    layer: *layer,
                    opacity: opacity_obj.opacity,
                });
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
                let event = ChartEvent::Bms(BmsEvent::BgaArgbChange {
                    layer: *layer,
                    argb: argb_obj.argb,
                });
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
            let event = ChartEvent::Bms(BmsEvent::BgmVolumeChange {
                volume: bgm_volume_obj.volume,
            });
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
            let event = ChartEvent::Bms(BmsEvent::KeyVolumeChange {
                volume: key_volume_obj.volume,
            });
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
            let event = ChartEvent::Bms(BmsEvent::TextDisplay {
                text: text_obj.text.clone(),
            });
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
            let event = ChartEvent::Bms(BmsEvent::JudgeLevelChange {
                level: judge_obj.judge_level,
            });
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        for seek_obj in bms.video.seek_events.values() {
            let y = get_event_y(seek_obj.time);
            let event = ChartEvent::Bms(BmsEvent::VideoSeek {
                seek_time: seek_obj.position.as_f64(),
            });
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        for bga_keybound_obj in bms.bmp.bga_keybound_events.values() {
            let y = get_event_y(bga_keybound_obj.time);
            let event = ChartEvent::Bms(BmsEvent::BgaKeybound {
                event: bga_keybound_obj.event.clone(),
            });
            events_map.entry(y).or_default().push(PlayheadEvent::new(
                id_gen.next_id(),
                y,
                event,
                TimeSpan::ZERO,
            ));
        }

        for option_obj in bms.option.option_events.values() {
            let y = get_event_y(option_obj.time);
            let event = ChartEvent::Bms(BmsEvent::OptionChange {
                option: option_obj.option.clone(),
            });
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
        .chain(
            bms.bpm
                .bpm_changes_u8
                .iter()
                .filter_map(|(obj_time, &bpm_u8)| {
                    // Value 0 = no event (same as flow_events), avoids junk BPM changes polluting the timeline
                    if bpm_u8 == 0 {
                        return None;
                    }
                    let y = y_memo.get_y(*obj_time);
                    let bpm = PositiveF64::new(bpm_u8 as f64).unwrap_or(DEFAULT_BPM);
                    Some((y, bpm))
                }),
        )
        .collect();
    points.extend(bpm_changes.iter().map(|(y, _)| *y));

    let stop_list: Vec<(YCoordinate, NonNegativeF64)> = bms
        .stop
        .stops
        .values()
        .map(|st| {
            let sy = y_memo.get_y(st.time);
            // 192nd note → beats (1 value = 1/48 beat, matching `ChartEvent::Stop`)
            (sy, convert_stop_duration_to_beats(st.duration))
        })
        .sorted_by_key(|(y, _)| *y)
        .collect();

    let cum_map = calculate_cumulative_times(
        &points,
        init_bpm_value,
        &bpm_changes,
        &stop_list,
        StopDurationUnit::Beats,
    );

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
    let lane = BmsProcessor::lane_of_channel_id::<T>(obj.channel_id);
    let wav_id = Some(WavId::from(obj.wav_id.as_u16() as usize));
    let Some((side, key, kind)) = lane else {
        return ChartEvent::Bgm { wav_id };
    };
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

// ---- BaseBpmGenerator implementations for BMS ----

use crate::chart::player::base_bpm::{
    BaseBpm, BaseBpmGenerator, MaxBpmGenerator, MinBpmGenerator, StartBpmGenerator,
};

impl BaseBpmGenerator<Bms> for StartBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .as_ref()
            .and_then(|bpm| bpm.value().as_ref().ok().copied())
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for MinBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .iter()
            .filter_map(|bpm| bpm.value().as_ref().ok().copied())
            .chain(bms.bpm.bpm_changes.values().map(|change| change.bpm))
            .min()
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for MaxBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .iter()
            .filter_map(|bpm| bpm.value().as_ref().ok().copied())
            .chain(bms.bpm.bpm_changes.values().map(|change| change.bpm))
            .max()
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for crate::chart::player::base_bpm::ManualBpmGenerator {
    fn generate(&self, _bms: &Bms) -> Option<BaseBpm> {
        Some(self.0)
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

    /// Test that BPM changes from channel 03 (u8) are converted to `ChartEvent::BpmChange`
    /// and `FlowEvent::Bpm` in the resulting Chart.
    #[test]
    fn test_bpm_changes_u8_converted_to_chart_events() {
        let mut bms = Bms::default();
        bms.bpm
            .bpm_changes_u8
            .insert(ObjTime::start_of(Track(1)), 150);
        bms.bpm
            .bpm_changes_u8
            .insert(ObjTime::start_of(Track(2)), 200);

        let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms)
            .expect("Parse should succeed with valid u8 BPM changes");

        // Verify ChartEvent::BpmChange events
        let bpm_values: Vec<PositiveF64> = chart
            .events()
            .as_events()
            .iter()
            .filter_map(|evp| {
                if let ChartEvent::BpmChange { bpm } = evp.event() {
                    Some(*bpm)
                } else {
                    None
                }
            })
            .collect();

        assert!(
            bpm_values.contains(&PositiveF64::new_const(150.0)),
            "Should contain BPM 150 from channel 03, got: {bpm_values:?}"
        );
        assert!(
            bpm_values.contains(&PositiveF64::new_const(200.0)),
            "Should contain BPM 200 from channel 03, got: {bpm_values:?}"
        );

        // Verify FlowEvent::Bpm events (used by step_to engine)
        let flow_bpm_values: Vec<PositiveF64> = chart
            .flow_events()
            .values()
            .flatten()
            .filter_map(|fe| {
                if let FlowEvent::Bpm(bpm) = fe {
                    Some(*bpm)
                } else {
                    None
                }
            })
            .collect();

        assert!(
            flow_bpm_values.contains(&PositiveF64::new_const(150.0)),
            "Should contain FlowEvent::Bpm 150 from channel 03, got: {flow_bpm_values:?}"
        );
        assert!(
            flow_bpm_values.contains(&PositiveF64::new_const(200.0)),
            "Should contain FlowEvent::Bpm 200 from channel 03, got: {flow_bpm_values:?}"
        );
    }
}
