//! Bms Processor Module.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Duration;

use gametime::TimeSpan;
use num::{ToPrimitive, Zero};

use crate::bms::Decimal;
use crate::bms::prelude::*;
use crate::chart_process::player::UniversalChartPlayer;
use crate::chart_process::resource::{HashMapResourceMapping, ResourceMapping};
use crate::chart_process::types::{
    AllEventsIndex, BmpId, ChartEventIdGenerator, PlayheadEvent, VisibleRangePerBpm, WavId,
    YCoordinate,
};
use crate::chart_process::y_calculator::BmsYCalculator;

use super::EventParseOutput;
use super::core::FlowEvent;

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// `ChartProcessor` of Bms files.
///
/// This processor parses BMS charts and produces an `EventParseOutput`.
/// Use the `to_player()` method to convert the parse output into a playable chart.
pub struct BmsProcessor<'a, T: KeyLayoutMapper> {
    /// Phantom data for the key layout mapper type
    _phantom: std::marker::PhantomData<&'a T>,

    /// Parsed chart output
    output: EventParseOutput,
}

impl<'a, T: KeyLayoutMapper> BmsProcessor<'a, T> {
    /// Create processor by parsing BMS chart.
    #[must_use]
    pub fn new(bms: &'a Bms) -> Self {
        // Create Y coordinate calculator
        let y_calc = BmsYCalculator::from_bms(bms);

        // Get initial BPM
        let init_bpm = bms
            .bpm
            .bpm
            .as_ref()
            .cloned()
            .unwrap_or_else(|| Decimal::from(120));

        // Precompute all events (without activate_time initially)
        let events_map = Self::precompute_events(bms, &y_calc);

        // Precompute activate times
        let all_events = Self::precompute_activate_times(bms, &events_map, &y_calc);

        // Build flow events mapping
        let flow_events_by_y = Self::build_flow_events(bms, &y_calc);

        // Build resource mapping
        let resources = Box::new(Self::build_resources(bms));

        let output = EventParseOutput {
            all_events,
            flow_events_by_y,
            init_bpm,
            resources,
        };

        Self {
            _phantom: std::marker::PhantomData,
            output,
        }
    }

    /// Convert the parse output into a playable chart.
    ///
    /// # Arguments
    /// * `visible_range_per_bpm` - Visible range configuration for playback
    #[must_use]
    pub fn to_player(
        self,
        visible_range_per_bpm: VisibleRangePerBpm,
    ) -> UniversalChartPlayer<HashMapResourceMapping> {
        UniversalChartPlayer::from_parse_output(self.output, visible_range_per_bpm)
    }

    /// Get access to all parsed events.
    #[must_use]
    pub const fn all_events(&self) -> &AllEventsIndex {
        &self.output.all_events
    }

    /// Get the initial BPM.
    #[must_use]
    pub const fn init_bpm(&self) -> &Decimal {
        &self.output.init_bpm
    }

    /// Get access to the resource mapping.
    #[must_use]
    pub fn resources(&self) -> &dyn ResourceMapping {
        self.output.resources.as_ref()
    }

    /// Precompute all events from BMS chart.
    fn precompute_events(
        bms: &Bms,
        y_memo: &BmsYCalculator,
    ) -> BTreeMap<YCoordinate, Vec<PlayheadEvent>> {
        let mut events_map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        let mut id_gen: ChartEventIdGenerator = ChartEventIdGenerator::default();

        // Note / Wav arrival events
        for obj in bms.notes().all_notes() {
            let y = y_memo.get_y(obj.offset);
            let event = Self::event_for_note(bms, y_memo, obj);

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // BPM change events
        for change in bms.bpm.bpm_changes.values() {
            let y = y_memo.get_y(change.time);
            let event = crate::chart_process::ChartEvent::BpmChange {
                bpm: change.bpm.clone(),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Scroll change events
        for change in bms.scroll.scrolling_factor_changes.values() {
            let y = y_memo.get_y(change.time);
            let event = crate::chart_process::ChartEvent::ScrollChange {
                factor: change.factor.clone(),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Speed change events
        for change in bms.speed.speed_factor_changes.values() {
            let y = y_memo.get_y(change.time);
            let event = crate::chart_process::ChartEvent::SpeedChange {
                factor: change.factor.clone(),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Stop events
        for stop in bms.stop.stops.values() {
            let y = y_memo.get_y(stop.time);
            let event = crate::chart_process::ChartEvent::Stop {
                duration: stop.duration.clone(),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // BGA change events
        for bga_obj in bms.bmp.bga_changes.values() {
            let y = y_memo.get_y(bga_obj.time);
            let bmp_index = bga_obj.id.as_u16() as usize;
            let event = crate::chart_process::ChartEvent::BgaChange {
                layer: bga_obj.layer,
                bmp_id: Some(BmpId::from(bmp_index)),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // BGA opacity change events
        for (layer, opacity_changes) in &bms.bmp.bga_opacity_changes {
            for opacity_obj in opacity_changes.values() {
                let y = y_memo.get_y(opacity_obj.time);
                let event = crate::chart_process::ChartEvent::BgaOpacityChange {
                    layer: *layer,
                    opacity: opacity_obj.opacity,
                };

                let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
                events_map.entry(y).or_default().push(evp);
            }
        }

        // BGA ARGB color change events
        for (layer, argb_changes) in &bms.bmp.bga_argb_changes {
            for argb_obj in argb_changes.values() {
                let y = y_memo.get_y(argb_obj.time);
                let event = crate::chart_process::ChartEvent::BgaArgbChange {
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
            let event = crate::chart_process::ChartEvent::BgmVolumeChange {
                volume: bgm_volume_obj.volume,
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // KEY volume change events
        for key_volume_obj in bms.volume.key_volume_changes.values() {
            let y = y_memo.get_y(key_volume_obj.time);
            let event = crate::chart_process::ChartEvent::KeyVolumeChange {
                volume: key_volume_obj.volume,
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Text display events
        for text_obj in bms.text.text_events.values() {
            let y = y_memo.get_y(text_obj.time);
            let event = crate::chart_process::ChartEvent::TextDisplay {
                text: text_obj.text.clone(),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Judge level change events
        for judge_obj in bms.judge.judge_events.values() {
            let y = y_memo.get_y(judge_obj.time);
            let event = crate::chart_process::ChartEvent::JudgeLevelChange {
                level: judge_obj.judge_level,
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Video seek events
        for seek_obj in bms.video.seek_events.values() {
            let y = y_memo.get_y(seek_obj.time);
            let event = crate::chart_process::ChartEvent::VideoSeek {
                seek_time: seek_obj.position.to_string().parse::<f64>().unwrap_or(0.0),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // BGA key binding events
        for bga_keybound_obj in bms.bmp.bga_keybound_events.values() {
            let y = y_memo.get_y(bga_keybound_obj.time);
            let event = crate::chart_process::ChartEvent::BgaKeybound {
                event: bga_keybound_obj.event.clone(),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Option change events
        for option_obj in bms.option.option_events.values() {
            let y = y_memo.get_y(option_obj.time);
            let event = crate::chart_process::ChartEvent::OptionChange {
                option: option_obj.option.clone(),
            };

            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, TimeSpan::ZERO);
            events_map.entry(y).or_default().push(evp);
        }

        // Generate bar lines
        Self::generate_barlines(bms, y_memo, &mut events_map, &mut id_gen);

        events_map
    }

    /// Generate measure lines for BMS.
    fn generate_barlines(
        bms: &Bms,
        y_memo: &BmsYCalculator,
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
                let event = crate::chart_process::ChartEvent::BarLine;
                let evp =
                    PlayheadEvent::new(id_gen.next_id(), track_y.clone(), event, TimeSpan::ZERO);
                events_map.entry(track_y).or_default().push(evp);
            }
        }
    }

    /// Precompute absolute `activate_time` for all events based on BPM segmentation and Stops.
    fn precompute_activate_times(
        bms: &Bms,
        events_map: &BTreeMap<YCoordinate, Vec<PlayheadEvent>>,
        y_memo: &BmsYCalculator,
    ) -> AllEventsIndex {
        use std::collections::{BTreeMap, BTreeSet};

        let mut points: BTreeSet<YCoordinate> = BTreeSet::new();
        points.insert(YCoordinate::zero());
        points.extend(events_map.keys().cloned());

        let mut bpm_map: BTreeMap<YCoordinate, Decimal> = BTreeMap::new();
        let init_bpm = bms
            .bpm
            .bpm
            .as_ref()
            .cloned()
            .unwrap_or_else(|| Decimal::from(120));
        bpm_map.insert(YCoordinate::zero(), init_bpm.clone());

        let bpm_pairs: Vec<(YCoordinate, Decimal)> = bms
            .bpm
            .bpm_changes
            .values()
            .map(|change| {
                let y = y_memo.get_y(change.time);
                (y, change.bpm.clone())
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
                (sy, st.duration.clone())
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

        for curr in points {
            if curr <= prev {
                continue;
            }

            let delta_y = curr.clone() - prev.clone();
            let delta_y_value = delta_y.value();
            let delta_nanos = if cur_bpm > Decimal::zero() {
                let numerator =
                    delta_y_value * Decimal::from(240u64) * Decimal::from(NANOS_PER_SECOND);
                (numerator / cur_bpm).round().to_u64().unwrap_or(0)
            } else {
                0
            };

            total_nanos = total_nanos.saturating_add(delta_nanos);

            while let Some((sy, dur_y)) = stop_list.get(stop_idx) {
                if sy > &curr {
                    break;
                }
                if sy > &prev {
                    let bpm_at_stop = bpm_map
                        .range((std::ops::Bound::Unbounded, std::ops::Bound::Included(sy)))
                        .next_back()
                        .map(|(_, b)| b.clone())
                        .unwrap_or_else(|| init_bpm.clone());

                    let dur_nanos = if bpm_at_stop > Decimal::zero() {
                        let numerator =
                            dur_y.clone() * Decimal::from(240u64) * Decimal::from(NANOS_PER_SECOND);
                        (numerator / bpm_at_stop).round().to_u64().unwrap_or(0)
                    } else {
                        0
                    };

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

        let new_map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = events_map
            .iter()
            .map(|(y_coord, events)| {
                let at_nanos = cum_map.get(y_coord).copied().unwrap_or(0);
                let at = TimeSpan::from_duration(Duration::from_nanos(at_nanos));
                let new_events: Vec<_> = events
                    .iter()
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

    /// Build flow events mapping by Y coordinate.
    fn build_flow_events(
        bms: &Bms,
        y_memo: &BmsYCalculator,
    ) -> BTreeMap<YCoordinate, Vec<FlowEvent>> {
        let mut flow_events_by_y: BTreeMap<YCoordinate, Vec<FlowEvent>> = BTreeMap::new();

        for change in bms.bpm.bpm_changes.values() {
            let y = y_memo.get_y(change.time);
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Bpm(change.bpm.clone()));
        }

        for change in bms.scroll.scrolling_factor_changes.values() {
            let y = y_memo.get_y(change.time);
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Scroll(change.factor.clone()));
        }

        for change in bms.speed.speed_factor_changes.values() {
            let y = y_memo.get_y(change.time);
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Speed(change.factor.clone()));
        }

        flow_events_by_y
    }

    /// Build resource mapping.
    fn build_resources(bms: &Bms) -> HashMapResourceMapping {
        let wav_paths: HashMap<WavId, PathBuf> = bms
            .wav
            .wav_files
            .iter()
            .map(|(obj_id, path)| (WavId::from(obj_id.as_u16() as usize), path.clone()))
            .collect();

        let bmp_paths: HashMap<BmpId, PathBuf> = bms
            .bmp
            .bmp_files
            .iter()
            .map(|(obj_id, bmp)| (BmpId::from(obj_id.as_u16() as usize), bmp.file.clone()))
            .collect();

        HashMapResourceMapping::new(wav_paths, bmp_paths)
    }

    /// Get the lane information for a channel ID.
    fn lane_of_channel_id(channel_id: NoteChannelId) -> Option<(PlayerSide, Key, NoteKind)> {
        let map = channel_id.try_into_map::<T>()?;
        let side = map.side();
        let key = map.key();
        let kind = map.kind();
        Some((side, key, kind))
    }

    /// Create a `ChartEvent` for a note object.
    fn event_for_note(
        bms: &Bms,
        y_memo: &BmsYCalculator,
        obj: &WavObj,
    ) -> crate::chart_process::ChartEvent {
        let y = y_memo.get_y(obj.offset);
        let lane = Self::lane_of_channel_id(obj.channel_id);
        let wav_id = Some(WavId::from(obj.wav_id.as_u16() as usize));

        let Some((side, key, kind)) = lane else {
            return crate::chart_process::ChartEvent::Bgm { wav_id };
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

        crate::chart_process::ChartEvent::Note {
            side,
            key,
            kind,
            wav_id,
            length,
            continue_play: None,
        }
    }
}
