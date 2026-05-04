//! Bms Processor Module.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    convert::TryFrom,
    path::PathBuf,
};

use itertools::Itertools;
use strict_num_extended::{FinF64, PositiveF64};

use crate::bms::command::string_value::StringValue;
use crate::bms::parse::check_playing::PlayingError;
use crate::bms::prelude::*;
use crate::chart::event::{BmsEvent, ChartEvent, FlowEvent, PlayheadEvent};
use crate::chart::prelude::{TimeSpan, YCoordinate};
use crate::chart::process::{
    AllEventsIndex, BmpId, ChartEventIdGenerator, ChartResources, Process, WavId,
    calculate_cumulative_times,
};
use crate::chart::{Chart, DEFAULT_BPM, DEFAULT_SPEED, MAX_FIN_F64, MAX_NON_NEGATIVE_F64};
use strict_num_extended::NonNegativeF64;

/// BMS format parser.
///
/// This struct serves as a namespace for BMS parsing functions.
/// It parses BMS files and returns a `Chart` containing all precomputed data.
struct BmsProcessor;

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
    fn parse<T: BmsLayoutMapper>(bms: &Bms) -> Result<Chart, PlayingError> {
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
    fn generate_barlines_for_bms(
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

    fn lane_of_channel_id<T: BmsLayoutMapper>(
        channel_id: NoteChannelId,
    ) -> Option<(PlayerSide, Key, NoteKind)> {
        let map = channel_id.try_into_map::<T>()?;
        let side = map.side();
        let key = map.key();
        let kind = map.kind();
        Some((side, key, kind))
    }
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

struct EventAppender<'a> {
    bms: &'a Bms,
    events_map: &'a mut BTreeMap<YCoordinate, Vec<PlayheadEvent>>,
    id_gen: &'a mut ChartEventIdGenerator,
    get_event_y: &'a dyn Fn(ObjTime) -> YCoordinate,
}

impl AllEventsIndex {
    /// Precompute all events, store grouped by Y coordinate
    #[must_use]
    pub fn precompute_all_events<T: BmsLayoutMapper>(bms: &Bms, y_memo: &YMemo) -> Self {
        let mut events_map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        let mut id_gen: ChartEventIdGenerator = ChartEventIdGenerator::default();
        let get_event_y = |time: ObjTime| -> YCoordinate { y_memo.get_y(time) };

        {
            let mut appender = EventAppender {
                bms,
                events_map: &mut events_map,
                id_gen: &mut id_gen,
                get_event_y: &get_event_y,
            };
            appender.append_note_events::<T>(y_memo);
            appender.append_bpm_events();
            appender.append_scroll_events();
            appender.append_speed_events();
            appender.append_stop_events();
            appender.append_bga_events();
            appender.append_volume_events();
            appender.append_text_events();
            appender.append_judge_events();
            appender.append_video_events();
            appender.append_option_events();
        }

        BmsProcessor::generate_barlines_for_bms(bms, y_memo, &mut events_map, &mut id_gen);
        Self::new(events_map)
    }
}

impl EventAppender<'_> {
    fn push_event(&mut self, y: YCoordinate, event: ChartEvent) {
        let id = self.id_gen.next_id();
        self.events_map
            .entry(y)
            .or_default()
            .push(PlayheadEvent::new(id, y, event, TimeSpan::ZERO));
    }

    fn append_note_events<T: BmsLayoutMapper>(&mut self, y_memo: &YMemo) {
        let note_events: Vec<(YCoordinate, WavObj)> = self
            .bms
            .notes()
            .all_notes()
            .map(|obj| (y_memo.get_y(obj.offset), obj.clone()))
            .sorted_by(|(y1, _), (y2, _)| y1.cmp(y2))
            .collect();

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

        let mut ln_start_markers: HashSet<(PlayerSide, Key)> = HashSet::new();
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
                let event = event_for_note_static::<T>(self.bms, y_memo, obj);
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
                        ln_start_markers.remove(&lane_key);
                        continue;
                    }
                    if length.is_some() {
                        ln_start_markers.insert(lane_key);
                    }
                }
                self.push_event(*y, event);
            }
        }
    }

    fn append_bpm_events(&mut self) {
        for change in self.bms.bpm.bpm_changes.values() {
            let y = (self.get_event_y)(change.time);
            let event = ChartEvent::BpmChange { bpm: change.bpm };
            self.push_event(y, event);
        }
    }

    fn append_scroll_events(&mut self) {
        for change in self.bms.scroll.scrolling_factor_changes.values() {
            let y = (self.get_event_y)(change.time);
            let event = ChartEvent::ScrollChange {
                factor: change.factor,
            };
            self.push_event(y, event);
        }
    }

    fn append_speed_events(&mut self) {
        for change in self.bms.speed.speed_factor_changes.values() {
            let y = (self.get_event_y)(change.time);
            let event = ChartEvent::SpeedChange {
                factor: change.factor,
            };
            self.push_event(y, event);
        }
    }

    fn append_stop_events(&mut self) {
        for stop in self.bms.stop.stops.values() {
            let y = (self.get_event_y)(stop.time);
            let event = ChartEvent::Stop {
                duration: convert_stop_duration_to_beats(stop.duration),
            };
            self.push_event(y, event);
        }
    }

    fn append_bga_events(&mut self) {
        for bga_obj in self.bms.bmp.bga_changes.values() {
            let y = (self.get_event_y)(bga_obj.time);
            let bmp_index = bga_obj.id.as_u16() as usize;
            let event = ChartEvent::BgaChange {
                layer: bga_obj.layer,
                bmp_id: Some(BmpId::from(bmp_index)),
            };
            self.push_event(y, event);
        }

        for (layer, opacity_changes) in &self.bms.bmp.bga_opacity_changes {
            for opacity_obj in opacity_changes.values() {
                let y = (self.get_event_y)(opacity_obj.time);
                let event = ChartEvent::Bms(BmsEvent::BgaOpacityChange {
                    layer: *layer,
                    opacity: opacity_obj.opacity,
                });
                self.push_event(y, event);
            }
        }

        for (layer, argb_changes) in &self.bms.bmp.bga_argb_changes {
            for argb_obj in argb_changes.values() {
                let y = (self.get_event_y)(argb_obj.time);
                let event = ChartEvent::Bms(BmsEvent::BgaArgbChange {
                    layer: *layer,
                    argb: argb_obj.argb,
                });
                self.push_event(y, event);
            }
        }

        for bga_keybound_obj in self.bms.bmp.bga_keybound_events.values() {
            let y = (self.get_event_y)(bga_keybound_obj.time);
            let event = ChartEvent::Bms(BmsEvent::BgaKeybound {
                event: bga_keybound_obj.event.clone(),
            });
            self.push_event(y, event);
        }
    }

    fn append_volume_events(&mut self) {
        for bgm_volume_obj in self.bms.volume.bgm_volume_changes.values() {
            let y = (self.get_event_y)(bgm_volume_obj.time);
            let event = ChartEvent::Bms(BmsEvent::BgmVolumeChange {
                volume: bgm_volume_obj.volume,
            });
            self.push_event(y, event);
        }
        for key_volume_obj in self.bms.volume.key_volume_changes.values() {
            let y = (self.get_event_y)(key_volume_obj.time);
            let event = ChartEvent::Bms(BmsEvent::KeyVolumeChange {
                volume: key_volume_obj.volume,
            });
            self.push_event(y, event);
        }
    }

    fn append_text_events(&mut self) {
        for text_obj in self.bms.text.text_events.values() {
            let y = (self.get_event_y)(text_obj.time);
            let event = ChartEvent::Bms(BmsEvent::TextDisplay {
                text: text_obj.text.clone(),
            });
            self.push_event(y, event);
        }
    }

    fn append_judge_events(&mut self) {
        for judge_obj in self.bms.judge.judge_events.values() {
            let y = (self.get_event_y)(judge_obj.time);
            let event = ChartEvent::Bms(BmsEvent::JudgeLevelChange {
                level: judge_obj.judge_level,
            });
            self.push_event(y, event);
        }
    }

    fn append_video_events(&mut self) {
        for seek_obj in self.bms.video.seek_events.values() {
            let y = (self.get_event_y)(seek_obj.time);
            let event = ChartEvent::Bms(BmsEvent::VideoSeek {
                seek_time: seek_obj.position.as_f64(),
            });
            self.push_event(y, event);
        }
    }

    fn append_option_events(&mut self) {
        for option_obj in self.bms.option.option_events.values() {
            let y = (self.get_event_y)(option_obj.time);
            let event = ChartEvent::Bms(BmsEvent::OptionChange {
                option: option_obj.option.clone(),
            });
            self.push_event(y, event);
        }
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

    let cum_map = calculate_cumulative_times(&points, init_bpm_value, &bpm_changes, &stop_list);

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
pub fn event_for_note_static<T: BmsLayoutMapper>(
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

impl TryFrom<Bms> for Chart {
    type Error = PlayingError;

    fn try_from(bms: Bms) -> Result<Self, Self::Error> {
        bms.process()
    }
}

impl Process for Bms {
    type Error = PlayingError;

    fn process(&self) -> Result<Chart, Self::Error> {
        BmsProcessor::parse::<crate::bms::command::channel::mapper::BmsLayoutBeat>(self)
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
    use crate::bms::command::channel::mapper::BmsLayoutBeat;
    use crate::bms::command::string_value::StringValue;

    /// Test that parsing fails when BPM value is invalid (non-numeric string)
    #[test]
    fn test_parse_invalid_bpm() {
        // Create a BMS object with an invalid BPM value
        let mut bms = Bms::default();
        bms.bpm.bpm = Some(StringValue::new("invalid_bpm"));

        // Try to parse, should return InvalidBpm error
        let result = BmsProcessor::parse::<BmsLayoutBeat>(&bms);

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

        let result = BmsProcessor::parse::<BmsLayoutBeat>(&bms);

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

        let result = BmsProcessor::parse::<BmsLayoutBeat>(&bms);

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
        let result = BmsProcessor::parse::<BmsLayoutBeat>(&bms);

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

        let result = BmsProcessor::parse::<BmsLayoutBeat>(&bms);

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

        let result = BmsProcessor::parse::<BmsLayoutBeat>(&bms);

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

        let result = BmsProcessor::parse::<BmsLayoutBeat>(&bms);

        match result {
            Err(PlayingError::InvalidBpm { raw, .. }) => {
                assert_eq!(raw, invalid_value);
            }
            _ => panic!("Expected PlayingError::InvalidBpm"),
        }
    }
}
