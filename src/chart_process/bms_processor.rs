//! Bms Processor Module.

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use num::{ToPrimitive, Zero};
use strict_num_extended::FinF64;

use crate::bms::Decimal;
use crate::bms::prelude::*;
use crate::chart_process::ChartEvent;
use crate::chart_process::types::{
    AllEventsIndex, BmpId, ChartEventIdGenerator, ChartResources, FlowEvent, ParsedChart,
    PlayheadEvent, TimeSpan, WavId, YCoordinate,
};

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
        // Pre-calculate the Y coordinate by tracks
        let y_memo = YMemo::new(bms);

        // Initialize BPM: prefer chart initial BPM, otherwise 120
        let init_bpm = bms.bpm.bpm.as_ref().cloned().unwrap_or_else(|| {
            "120".to_string().parse().unwrap_or_else(|_| StringValue {
                string: "120".to_string(),
                value: Ok(FinF64::new(120.0).expect("Failed to create FinF64 from 120.0")),
            })
        });

        let all_events = AllEventsIndex::precompute_all_events::<T>(bms, &y_memo);

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

        // Pre-index flow events by y
        let mut flow_events_by_y: BTreeMap<YCoordinate, Vec<FlowEvent>> = BTreeMap::new();
        for change in bms.bpm.bpm_changes.values() {
            let y = y_memo.get_y(change.time);
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Bpm(change.bpm.as_f64().unwrap_or(120.0)));
        }
        for change in bms.scroll.scrolling_factor_changes.values() {
            let y = y_memo.get_y(change.time);
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Scroll(change.factor.as_f64().unwrap_or(1.0)));
        }
        for change in bms.speed.speed_factor_changes.values() {
            let y = y_memo.get_y(change.time);
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Speed(change.factor.as_f64().unwrap_or(1.0)));
        }

        // Precompute activate times
        let all_events = precompute_activate_times(bms, &all_events, &y_memo);

        ParsedChart::new(
            ChartResources::new(wav_files, bmp_files),
            all_events,
            flow_events_by_y,
            Decimal::from(init_bpm.as_f64().unwrap_or(120.0)),
            Decimal::from(1.0),
        )
    }

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
            let track_y = y_memo.get_y(ObjTime::start_of(track.into()));

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
}

impl YMemo {
    fn new(bms: &Bms) -> Self {
        let mut y_by_track: BTreeMap<Track, Decimal> = BTreeMap::new();
        let mut last_track = 0;
        let mut y = Decimal::zero();
        for (&track, section_len_change) in &bms.section_len.section_len_changes {
            let passed_sections = (track.0 - last_track).saturating_sub(1);
            y += Decimal::from(passed_sections);
            let length_decimal: Decimal =
                section_len_change
                    .length
                    .string
                    .parse()
                    .unwrap_or_else(|_| {
                        Decimal::from(section_len_change.length.as_f64().unwrap_or(1.0))
                    });
            y += length_decimal;
            y_by_track.insert(track, y.clone());
            last_track = track.0;
        }
        Self {
            y_by_track,
            speed_changes: bms.speed.speed_factor_changes.clone(),
        }
    }

    // Finds Y coordinate at `time` efficiently
    fn get_y(&self, time: ObjTime) -> YCoordinate {
        let section_y = {
            let track = time.track();
            if let Some((&last_track, last_y)) = self.y_by_track.range(..=&track).last() {
                let passed_sections = track.0 - last_track.0;
                &Decimal::from(passed_sections) + last_y
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
            .map_or_else(|| 1.0, |(_, obj)| obj.factor.as_f64().unwrap_or(1.0));
        YCoordinate((section_y + fraction) * factor)
    }
}

impl AllEventsIndex {
    /// Precompute all events, store grouped by Y coordinate
    /// Note: Speed effects are calculated into event positions during initialization, ensuring event trigger times remain unchanged
    #[must_use]
    pub fn precompute_all_events<T: KeyLayoutMapper>(bms: &Bms, y_memo: &YMemo) -> Self {
        let mut events_map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        let mut id_gen: ChartEventIdGenerator = ChartEventIdGenerator::default();

        // Note / Wav arrival events
        for obj in bms.notes().all_notes() {
            let y = y_memo.get_y(obj.offset);
            let event = event_for_note_static::<T>(bms, y_memo, obj);

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // BPM change events
        for change in bms.bpm.bpm_changes.values() {
            let y = y_memo.get_y(change.time);
            let event = ChartEvent::BpmChange {
                bpm: Decimal::from(change.bpm.as_f64().unwrap_or(120.0)),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Scroll change events
        for change in bms.scroll.scrolling_factor_changes.values() {
            let y = y_memo.get_y(change.time);
            let event = ChartEvent::ScrollChange {
                factor: Decimal::from(change.factor.as_f64().unwrap_or(1.0)),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Speed change events
        for change in bms.speed.speed_factor_changes.values() {
            let y = y_memo.get_y(change.time);
            let event = ChartEvent::SpeedChange {
                factor: Decimal::from(change.factor.as_f64().unwrap_or(1.0)),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Stop events
        for stop in bms.stop.stops.values() {
            let y = y_memo.get_y(stop.time);
            let duration_decimal = stop
                .duration
                .as_big_decimal()
                .unwrap_or_else(|| Decimal::from(0.0));
            let event = ChartEvent::Stop {
                duration: convert_stop_duration_to_beats(&duration_decimal),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // BGA change events
        for bga_obj in bms.bmp.bga_changes.values() {
            let y = y_memo.get_y(bga_obj.time);
            let bmp_index = bga_obj.id.as_u16() as usize;
            let event = ChartEvent::BgaChange {
                layer: bga_obj.layer,
                bmp_id: Some(BmpId::from(bmp_index)),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // BGA opacity change events (requires minor-command feature)

        for (layer, opacity_changes) in &bms.bmp.bga_opacity_changes {
            for opacity_obj in opacity_changes.values() {
                let y = y_memo.get_y(opacity_obj.time);
                let event = ChartEvent::BgaOpacityChange {
                    layer: *layer,
                    opacity: opacity_obj.opacity,
                };

                let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
                events_map.entry(y).or_default().push(evp);
            }
        }

        // BGA ARGB color change events (requires minor-command feature)
        for (layer, argb_changes) in &bms.bmp.bga_argb_changes {
            for argb_obj in argb_changes.values() {
                let y = y_memo.get_y(argb_obj.time);
                let event = ChartEvent::BgaArgbChange {
                    layer: *layer,
                    argb: argb_obj.argb,
                };

                let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
                events_map.entry(y).or_default().push(evp);
            }
        }

        // BGM volume change events
        for bgm_volume_obj in bms.volume.bgm_volume_changes.values() {
            let y = y_memo.get_y(bgm_volume_obj.time);
            let event = ChartEvent::BgmVolumeChange {
                volume: bgm_volume_obj.volume,
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // KEY volume change events
        for key_volume_obj in bms.volume.key_volume_changes.values() {
            let y = y_memo.get_y(key_volume_obj.time);
            let event = ChartEvent::KeyVolumeChange {
                volume: key_volume_obj.volume,
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Text display events
        for text_obj in bms.text.text_events.values() {
            let y = y_memo.get_y(text_obj.time);
            let event = ChartEvent::TextDisplay {
                text: text_obj.text.clone(),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Judge level change events
        for judge_obj in bms.judge.judge_events.values() {
            let y = y_memo.get_y(judge_obj.time);
            let event = ChartEvent::JudgeLevelChange {
                level: judge_obj.judge_level,
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Minor-command feature events

        {
            // Video seek events
            for seek_obj in bms.video.seek_events.values() {
                let y = y_memo.get_y(seek_obj.time);
                let event = ChartEvent::VideoSeek {
                    seek_time: seek_obj.position.to_string().parse::<f64>().unwrap_or(0.0),
                };

                let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
                events_map.entry(y).or_default().push(evp);
            }

            // BGA key binding events
            for bga_keybound_obj in bms.bmp.bga_keybound_events.values() {
                let y = y_memo.get_y(bga_keybound_obj.time);
                let event = ChartEvent::BgaKeybound {
                    event: bga_keybound_obj.event.clone(),
                };

                let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
                events_map.entry(y).or_default().push(evp);
            }

            // Option change events
            for option_obj in bms.option.option_events.values() {
                let y = y_memo.get_y(option_obj.time);
                let event = ChartEvent::OptionChange {
                    option: option_obj.option.clone(),
                };

                let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
                events_map.entry(y).or_default().push(evp);
            }
        }

        BmsProcessor::generate_barlines_for_bms(bms, y_memo, &mut events_map, &mut id_gen);
        let pre_index = Self::new(events_map);
        precompute_activate_times(bms, &pre_index, y_memo)
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

    let mut bpm_map: BTreeMap<YCoordinate, Decimal> = BTreeMap::new();
    let init_bpm = bms
        .bpm
        .bpm
        .as_ref()
        .cloned()
        .and_then(|sv| sv.as_big_decimal())
        .unwrap_or_else(|| Decimal::from(120.0));
    bpm_map.insert(YCoordinate::zero(), init_bpm.clone());
    let bpm_pairs: Vec<(YCoordinate, Decimal)> = bms
        .bpm
        .bpm_changes
        .values()
        .map(|change| {
            let y = y_memo.get_y(change.time);
            let bpm_decimal = change
                .bpm
                .string
                .parse()
                .unwrap_or_else(|_| change.bpm.as_f64().unwrap_or(120.0).into());
            (y, bpm_decimal)
        })
        .collect();
    bpm_map.extend(bpm_pairs.iter().cloned());
    points.extend(bpm_pairs.iter().map(|(y, _)| y.clone()));

    let mut stop_list: Vec<(YCoordinate, Decimal)> = bms
        .stop
        .stops
        .values()
        .map(|st| {
            let sy = y_memo.get_y(st.time);
            let stop_decimal = st
                .duration
                .string
                .parse()
                .unwrap_or_else(|_| st.duration.as_f64().unwrap_or(0.0).into());
            (sy, stop_decimal)
        })
        .collect();
    stop_list.sort_by(|a, b| a.0.cmp(&b.0));

    let mut cum_map: BTreeMap<YCoordinate, u64> = BTreeMap::new();
    let mut total_nanos: u64 = 0;
    let mut prev = YCoordinate::zero();
    cum_map.insert(prev.clone(), 0);
    let mut cur_bpm = bpm_map
        .range((std::ops::Bound::Unbounded, std::ops::Bound::Included(&prev)))
        .next_back()
        .map(|(_, b)| b.clone())
        .unwrap_or_else(|| init_bpm.clone());
    let mut stop_idx = 0usize;
    for curr in points.into_iter() {
        if curr <= prev {
            continue;
        }
        let delta_y = curr.clone() - prev.clone();
        let delta_y_value = delta_y.value();
        let delta_nanos = (delta_y_value * Decimal::from(240u64) * Decimal::from(NANOS_PER_SECOND)
            / cur_bpm)
            .round()
            .to_u64()
            .unwrap_or(0);
        total_nanos = total_nanos.saturating_add(delta_nanos);
        while let Some((sy, dur_y)) = stop_list.get(stop_idx) {
            if sy >= &curr {
                break;
            }
            if sy > &prev {
                let bpm_at_stop = bpm_map
                    .range((std::ops::Bound::Unbounded, std::ops::Bound::Included(sy)))
                    .next_back()
                    .map(|(_, b)| b.clone())
                    .unwrap_or_else(|| init_bpm.clone());
                let dur_nanos =
                    (dur_y.clone() * Decimal::from(240u64) * Decimal::from(NANOS_PER_SECOND)
                        / bpm_at_stop)
                        .round()
                        .to_u64()
                        .unwrap_or(0);
                total_nanos = total_nanos.saturating_add(dur_nanos);
            }
            stop_idx += 1;
        }
        cur_bpm = bpm_map
            .range((std::ops::Bound::Unbounded, std::ops::Bound::Included(&curr)))
            .next_back()
            .map(|(_, b)| b.clone())
            .unwrap_or_else(|| init_bpm.clone());
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
