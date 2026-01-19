//! Bms Processor Module.

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use itertools::Itertools;
use num::{One, ToPrimitive, Zero};

use crate::bms::Decimal;
use crate::bms::prelude::*;
use crate::chart_process::processor::{
    AllEventsIndex, BmpId, ChartEventIdGenerator, ChartResources, ParsedChart, WavId,
};
use crate::chart_process::{ChartEvent, FlowEvent, PlayheadEvent, TimeSpan, YCoordinate};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// BMS format parser.
///
/// This struct serves as a namespace for BMS parsing functions.
/// It parses BMS files and returns a `ParsedChart` containing all precomputed data.
pub struct BmsProcessor;

/// Convert STOP duration from 192nd-note units to beats (measure units).
///
/// In 4/4 time signature:
/// - 192nd-note represents 1/192 of a whole note
/// - One measure (4/4) = 4 beats = 192/48 beats
/// - Therefore: 1 unit of 192nd-note = 1/48 beat
/// - Formula: beats = `192nd_note_value` / 48
#[must_use]
fn convert_stop_duration_to_beats(duration_192nd: &Decimal) -> Decimal {
    duration_192nd.clone() / Decimal::from(48)
}

impl BmsProcessor {
    /// Parse BMS file and return a `ParsedChart` containing all precomputed data.
    #[must_use]
    pub fn parse<T: KeyLayoutMapper>(bms: &Bms) -> ParsedChart {
        // Pre-calculate Y coordinate by tracks
        let y_memo = YMemo::new(bms);

        // Initialize BPM: prefer chart initial BPM, otherwise 120
        let init_bpm = bms
            .bpm
            .bpm
            .as_ref()
            .cloned()
            .unwrap_or_else(|| Decimal::from(120));

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
        let all_events = precompute_activate_times(bms, &all_events, &y_memo);

        ParsedChart::new(
            ChartResources::new(wav_files, bmp_files),
            all_events,
            y_memo.flow_events().clone(),
            init_bpm,
            Decimal::one(),
        )
    }

    /// Generate measure lines for BMS (generated for each track, but not exceeding other objects' Y values)
    /// Generate measure lines for BMS (generated for each track, but not exceeding other objects' Y values)
    pub fn generate_barlines_for_bms(
        bms: &Bms,
        y_memo: &YMemo,
        events_map: &mut BTreeMap<YCoordinate, Vec<PlayheadEvent>>,
        id_gen: &mut ChartEventIdGenerator,
    ) {
        // Find the maximum Y value of all events
        let Some(max_y) = events_map.last_key_value().map(|(key, _)| key.clone()) else {
            return;
        };

        if max_y.0 <= Decimal::zero() {
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
                let evp =
                    PlayheadEvent::new(id_gen.next_id(), track_y.clone(), event, TimeSpan::ZERO);
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

/// Y coordinate memoization for efficient position calculation.
///
/// This structure caches Y coordinate calculations by track, accounting for
/// section length changes and speed modifications.
#[derive(Debug)]
pub struct YMemo {
    /// Y coordinates memoization by track, which modified its length
    y_by_track: BTreeMap<Track, Decimal>,
    speed_changes: BTreeMap<ObjTime, SpeedObj>,
    zero_length_tracks: std::collections::HashSet<Track>,
    /// Flow events that affect playback speed/scroll, organized by Y coordinate
    flow_events: BTreeMap<YCoordinate, Vec<FlowEvent>>,
}

impl YMemo {
    fn new(bms: &Bms) -> Self {
        let mut y_by_track: BTreeMap<Track, Decimal> = BTreeMap::new();
        let mut last_track = 0;
        let mut y = Decimal::zero();
        for (&track, section_len_change) in &bms.section_len.section_len_changes {
            let passed_sections = (track.0 - last_track).saturating_sub(1);
            y += Decimal::from(passed_sections);
            y += &section_len_change.length;
            y_by_track.insert(track, y.clone());
            last_track = track.0;
        }

        let zero_length_tracks: std::collections::HashSet<Track> = bms
            .section_len
            .section_len_changes
            .iter()
            .filter(|(_, change)| change.length.is_zero())
            .map(|(&track, _)| track)
            .collect();

        // Populate flow events by Y coordinate
        let get_event_y = |time: ObjTime| -> YCoordinate {
            let section_y =
                if let Some((&track_last, track_y)) = y_by_track.range(..=&time.track()).last() {
                    let passed_sections = (time.track().0 - track_last.0).saturating_sub(1);
                    Decimal::from(passed_sections) + track_y.clone()
                } else {
                    Decimal::from(time.track().0)
                };
            let fraction = if time.denominator().get() > 0 {
                Decimal::from(time.numerator()) / Decimal::from(time.denominator().get())
            } else {
                Default::default()
            };
            let factor = bms
                .speed
                .speed_factor_changes
                .range(..=time)
                .last()
                .map_or_else(Decimal::one, |(_, obj)| obj.factor.clone());
            YCoordinate((section_y + fraction) * factor)
        };

        let mut flow_events: BTreeMap<YCoordinate, Vec<FlowEvent>> = BTreeMap::new();

        // BPM changes
        for change in bms.bpm.bpm_changes.values() {
            let event_y = get_event_y(change.time);
            flow_events
                .entry(event_y)
                .or_default()
                .push(FlowEvent::Bpm(change.bpm.clone()));
        }

        // Scroll changes
        for change in bms.scroll.scrolling_factor_changes.values() {
            let event_y = get_event_y(change.time);
            flow_events
                .entry(event_y)
                .or_default()
                .push(FlowEvent::Scroll(change.factor.clone()));
        }

        // Speed changes
        for change in bms.speed.speed_factor_changes.values() {
            let event_y = get_event_y(change.time);
            flow_events
                .entry(event_y)
                .or_default()
                .push(FlowEvent::Speed(change.factor.clone()));
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
                Decimal::from(passed_sections) + last_y.clone()
            } else {
                // there is no sections modified its length until
                Decimal::from(track.0)
            }
        };
        let fraction = if time.denominator().get() > 0 {
            Decimal::from(time.numerator()) / Decimal::from(time.denominator().get())
        } else {
            Default::default()
        };
        let factor = self
            .speed_changes
            .range(..=time)
            .last()
            .map_or_else(Decimal::one, |(_, obj)| obj.factor.clone());
        YCoordinate((section_y + fraction) * factor)
    }

    // Gets the Y coordinate at the start of a track/section (without fraction)
    fn get_section_start_y(&self, track: Track) -> YCoordinate {
        let section_y = if let Some((&last_track, last_y)) = self.y_by_track.range(..=&track).last()
        {
            let passed_sections = track.0 - last_track.0;
            Decimal::from(passed_sections) + last_y.clone()
        } else {
            Decimal::from(track.0)
        };
        let factor = self
            .speed_changes
            .range(..=ObjTime::start_of(track))
            .last()
            .map_or_else(Decimal::one, |(_, obj)| obj.factor.clone());
        YCoordinate(section_y * factor)
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
            .collect();

        let mut zero_length_key_tracker: std::collections::HashMap<
            (YCoordinate, (PlayerSide, Key)),
            usize,
        > = std::collections::HashMap::new();

        for (i, (y, obj)) in note_events.iter().enumerate() {
            let is_zero_length_section = y_memo.zero_length_tracks.contains(&obj.offset.track());
            let lane = BmsProcessor::lane_of_channel_id::<T>(obj.channel_id);

            if let Some((side, key, _)) = lane
                && is_zero_length_section
            {
                zero_length_key_tracker.insert((y.clone(), (side, key)), i);
            }
        }

        for (i, (y, obj)) in note_events.iter().enumerate() {
            let is_zero_length_section = y_memo.zero_length_tracks.contains(&obj.offset.track());
            let lane = BmsProcessor::lane_of_channel_id::<T>(obj.channel_id);
            let should_include = match lane {
                Some((side, key, _)) if is_zero_length_section => {
                    zero_length_key_tracker.get(&(y.clone(), (side, key))) == Some(&i)
                }
                _ => true,
            };

            if should_include {
                let event = event_for_note_static::<T>(bms, y_memo, obj);
                let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
                events_map.entry(y.clone()).or_default().push(evp);
            }
        }

        for change in bms.bpm.bpm_changes.values() {
            let y = get_event_y(change.time);
            let event = ChartEvent::BpmChange {
                bpm: change.bpm.clone(),
            };
            events_map
                .entry(y.clone())
                .or_default()
                .push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y.clone(),
                    event,
                    TimeSpan::ZERO,
                ));
        }

        // Scroll change events
        for change in bms.scroll.scrolling_factor_changes.values() {
            let y = get_event_y(change.time);
            let event = ChartEvent::ScrollChange {
                factor: change.factor.clone(),
            };
            events_map
                .entry(y.clone())
                .or_default()
                .push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y.clone(),
                    event,
                    TimeSpan::ZERO,
                ));
        }

        // Speed change events
        for change in bms.speed.speed_factor_changes.values() {
            let y = get_event_y(change.time);
            let event = ChartEvent::SpeedChange {
                factor: change.factor.clone(),
            };
            events_map
                .entry(y.clone())
                .or_default()
                .push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y.clone(),
                    event,
                    TimeSpan::ZERO,
                ));
        }

        // Stop events
        for stop in bms.stop.stops.values() {
            let y = get_event_y(stop.time);
            let event = ChartEvent::Stop {
                duration: convert_stop_duration_to_beats(&stop.duration),
            };
            events_map
                .entry(y.clone())
                .or_default()
                .push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y.clone(),
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
            events_map
                .entry(y.clone())
                .or_default()
                .push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y.clone(),
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
                events_map
                    .entry(y.clone())
                    .or_default()
                    .push(PlayheadEvent::new(
                        id_gen.next_id(),
                        y.clone(),
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
                events_map
                    .entry(y.clone())
                    .or_default()
                    .push(PlayheadEvent::new(
                        id_gen.next_id(),
                        y.clone(),
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
            events_map
                .entry(y.clone())
                .or_default()
                .push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y.clone(),
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
            events_map
                .entry(y.clone())
                .or_default()
                .push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y.clone(),
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
            events_map
                .entry(y.clone())
                .or_default()
                .push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y.clone(),
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
            events_map
                .entry(y.clone())
                .or_default()
                .push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y.clone(),
                    event,
                    TimeSpan::ZERO,
                ));
        }

        for seek_obj in bms.video.seek_events.values() {
            let y = get_event_y(seek_obj.time);
            let event = ChartEvent::VideoSeek {
                seek_time: seek_obj.position.to_string().parse::<f64>().unwrap_or(0.0),
            };
            events_map
                .entry(y.clone())
                .or_default()
                .push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y.clone(),
                    event,
                    TimeSpan::ZERO,
                ));
        }

        for bga_keybound_obj in bms.bmp.bga_keybound_events.values() {
            let y = get_event_y(bga_keybound_obj.time);
            let event = ChartEvent::BgaKeybound {
                event: bga_keybound_obj.event.clone(),
            };
            events_map
                .entry(y.clone())
                .or_default()
                .push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y.clone(),
                    event,
                    TimeSpan::ZERO,
                ));
        }

        for option_obj in bms.option.option_events.values() {
            let y = get_event_y(option_obj.time);
            let event = ChartEvent::OptionChange {
                option: option_obj.option.clone(),
            };
            events_map
                .entry(y.clone())
                .or_default()
                .push(PlayheadEvent::new(
                    id_gen.next_id(),
                    y.clone(),
                    event,
                    TimeSpan::ZERO,
                ));
        }

        BmsProcessor::generate_barlines_for_bms(bms, y_memo, &mut events_map, &mut id_gen);
        Self::new(events_map)
    }
}

/// Precompute absolute `activate_time` for all events based on BPM segmentation and Stops.
#[must_use]
pub fn precompute_activate_times(
    bms: &Bms,
    all_events: &AllEventsIndex,
    y_memo: &YMemo,
) -> AllEventsIndex {
    use std::collections::{BTreeMap, BTreeSet};
    let mut points: BTreeSet<YCoordinate> = BTreeSet::new();
    points.insert(YCoordinate::zero());
    points.extend(all_events.as_by_y().keys().cloned());

    let init_bpm = bms
        .bpm
        .bpm
        .as_ref()
        .cloned()
        .unwrap_or_else(|| Decimal::from(120));
    let bpm_changes: Vec<(YCoordinate, Decimal)> = bms
        .bpm
        .bpm_changes
        .values()
        .map(|change| {
            let y = y_memo.get_y(change.time);
            (y, change.bpm.clone())
        })
        .collect();
    points.extend(bpm_changes.iter().map(|(y, _)| y.clone()));

    let stop_list: Vec<(YCoordinate, Decimal)> = bms
        .stop
        .stops
        .values()
        .map(|st| {
            let sy = y_memo.get_y(st.time);
            (sy, st.duration.clone())
        })
        .sorted_by_key(|(y, _)| y.clone())
        .collect();

    let mut bpm_map: BTreeMap<YCoordinate, Decimal> = BTreeMap::new();
    bpm_map.insert(YCoordinate::zero(), init_bpm.clone());
    bpm_map.extend(bpm_changes.iter().cloned());

    let mut cum_map: BTreeMap<YCoordinate, u64> = BTreeMap::new();
    let mut total_nanos: u64 = 0;
    let mut prev = YCoordinate::zero();
    cum_map.insert(prev.clone(), 0);
    let mut cur_bpm = init_bpm.clone();
    let mut stop_idx = 0usize;

    for curr in points {
        if curr <= prev {
            continue;
        }

        if let Some((_, bpm)) = bpm_map.range(..=&curr).next_back() {
            cur_bpm = bpm.clone();
        }

        let delta_y = curr.clone() - prev.clone();
        let delta_nanos =
            (delta_y.value() * Decimal::from(240u64) * Decimal::from(NANOS_PER_SECOND)
                / cur_bpm.clone())
            .to_u64()
            .unwrap_or(0);
        total_nanos = total_nanos.saturating_add(delta_nanos);

        while let Some((sy, dur_y)) = stop_list.get(stop_idx) {
            if sy >= &curr {
                break;
            }
            if sy > &prev {
                let bpm_at_stop = bpm_map
                    .range(..=sy)
                    .next_back()
                    .map(|(_, b)| b.clone())
                    .unwrap_or_else(|| init_bpm.clone());
                let dur_nanos = (dur_y * Decimal::from(240u64) * Decimal::from(NANOS_PER_SECOND)
                    / bpm_at_stop)
                    .to_u64()
                    .unwrap_or(0);
                total_nanos = total_nanos.saturating_add(dur_nanos);
            }
            stop_idx += 1;
        }

        cum_map.insert(curr.clone(), total_nanos);
        prev = curr;
    }

    let new_map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = all_events
        .as_by_y()
        .iter()
        .map(|(y_coord, indices)| {
            let at_nanos = cum_map.get(y_coord).copied().unwrap_or(0);
            let at = TimeSpan::from_duration(std::time::Duration::from_nanos(at_nanos));
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
            (y_coord.clone(), new_events)
        })
        .collect();
    AllEventsIndex::new(new_map)
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
                    next_y - y
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
