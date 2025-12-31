//! Chart parser module
//!
//! Provides traits and implementations for parsing different chart formats
//! into a unified representation of `PlayheadEvent` lists.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Duration;

use gametime::TimeSpan;
use num::{One, ToPrimitive, Zero};

use crate::bms::Decimal;
use crate::bms::prelude::*;
#[cfg(feature = "bmson")]
use crate::bmson::prelude::*;
use crate::util::StrExtension;

use super::core::FlowEvent;
use super::resource::{HashMapResourceMapping, NameBasedResourceMapping, ResourceMapping};
use super::types::{
    AllEventsIndex, BmpId, ChartEventIdGenerator, PlayheadEvent, WavId, YCoordinate,
};
use super::y_calculator::BmsYCalculator;

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Output of chart parsing.
///
/// Contains all the information needed for chart playback.
pub struct EventParseOutput {
    /// All events with their positions and activation times
    pub all_events: AllEventsIndex,

    /// Flow events (BPM/Speed/Scroll changes) indexed by Y coordinate
    pub flow_events_by_y: BTreeMap<YCoordinate, Vec<FlowEvent>>,

    /// Initial BPM
    pub init_bpm: Decimal,

    /// Resource mapping
    pub resources: Box<dyn ResourceMapping>,
}

/// Chart parser trait.
///
/// Defines the interface for parsing different chart formats
/// into a unified `EventParseOutput`.
pub trait ChartParser {
    /// Parse the chart and generate event list.
    ///
    /// Returns an `EventParseOutput` containing all events and metadata.
    fn parse(&self) -> EventParseOutput;
}

/// BMS chart parser.
///
/// Parses BMS format charts and generates `PlayheadEvent` lists.
pub struct BmsParser<'a, T: KeyLayoutMapper> {
    /// Reference to the BMS chart
    bms: &'a Bms,

    /// Phantom data for the key layout mapper type
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: KeyLayoutMapper> BmsParser<'a, T> {
    /// Create a new BMS parser.
    #[must_use]
    pub const fn new(bms: &'a Bms) -> Self {
        Self {
            bms,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a, T: KeyLayoutMapper> ChartParser for BmsParser<'a, T> {
    fn parse(&self) -> EventParseOutput {
        // Create Y coordinate calculator
        let y_calc = BmsYCalculator::from_bms(self.bms);

        // Get initial BPM
        let init_bpm = self
            .bms
            .bpm
            .bpm
            .as_ref()
            .cloned()
            .unwrap_or_else(|| Decimal::from(120));

        // Precompute all events (without activate_time initially)
        let events_map = Self::precompute_events(self.bms, &y_calc);

        // Precompute activate times
        let all_events = Self::precompute_activate_times(self.bms, &events_map, &y_calc);

        // Build flow events mapping
        let flow_events_by_y = Self::build_flow_events(self.bms, &y_calc);

        // Build resource mapping
        let resources = Box::new(Self::build_resources(self.bms));

        EventParseOutput {
            all_events,
            flow_events_by_y,
            init_bpm,
            resources,
        }
    }
}

impl<'a, T: KeyLayoutMapper> BmsParser<'a, T> {
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

/// BMSON chart parser.
///
/// Parses BMSON format charts and generates `PlayheadEvent` lists.
#[cfg(feature = "bmson")]
pub struct BmsonParser<'a> {
    /// Reference to the BMSON chart
    bmson: &'a Bmson<'a>,
}

#[cfg(feature = "bmson")]
impl<'a> BmsonParser<'a> {
    /// Create a new BMSON parser.
    #[must_use]
    pub const fn new(bmson: &'a Bmson<'a>) -> Self {
        Self { bmson }
    }
}

#[cfg(feature = "bmson")]
impl<'a> ChartParser for BmsonParser<'a> {
    fn parse(&self) -> EventParseOutput {
        let init_bpm: Decimal = self.bmson.info.init_bpm.as_f64().into();

        // Preprocess: assign IDs to all audio and image resources
        let mut audio_name_to_id = HashMap::new();
        let mut bmp_name_to_id = HashMap::new();
        let mut next_audio_id = 0usize;
        let mut next_bmp_id = 0usize;

        // Process audio files
        for sound_channel in &self.bmson.sound_channels {
            if let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(sound_channel.name.to_string())
            {
                e.insert(WavId::new(next_audio_id));
                next_audio_id += 1;
            }
        }

        // Process mine audio files
        for mine_channel in &self.bmson.mine_channels {
            if let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(mine_channel.name.to_string())
            {
                e.insert(WavId::new(next_audio_id));
                next_audio_id += 1;
            }
        }

        // Process hidden key audio files
        for key_channel in &self.bmson.key_channels {
            if let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(key_channel.name.to_string())
            {
                e.insert(WavId::new(next_audio_id));
                next_audio_id += 1;
            }
        }

        // Process image files
        for BgaHeader { name, .. } in &self.bmson.bga.bga_header {
            if let std::collections::hash_map::Entry::Vacant(e) =
                bmp_name_to_id.entry(name.to_string())
            {
                e.insert(BmpId::new(next_bmp_id));
                next_bmp_id += 1;
            }
        }

        // Precompute all events
        let all_events = Self::precompute_events(self.bmson, &audio_name_to_id, &bmp_name_to_id);

        // Build flow events mapping
        let flow_events_by_y = Self::build_flow_events(self.bmson);

        // Build resource mapping
        let resources = Box::new(NameBasedResourceMapping::new(
            audio_name_to_id,
            bmp_name_to_id,
        ));

        EventParseOutput {
            all_events,
            flow_events_by_y,
            init_bpm,
            resources,
        }
    }
}

#[cfg(feature = "bmson")]
impl<'a> BmsonParser<'a> {
    /// Precompute all events from BMSON chart.
    fn precompute_events(
        bmson: &Bmson<'a>,
        audio_name_to_id: &HashMap<String, WavId>,
        bmp_name_to_id: &HashMap<String, BmpId>,
    ) -> AllEventsIndex {
        use std::collections::BTreeSet;

        let denom = Decimal::from(4 * bmson.info.resolution.get());
        let denom_inv = if denom == Decimal::zero() {
            Decimal::zero()
        } else {
            Decimal::one() / denom
        };
        let pulses_to_y = |pulses: u64| {
            let pulses = Decimal::from(pulses);
            YCoordinate::new(&pulses * &denom_inv)
        };

        // Collect all Y points
        let mut points: BTreeSet<YCoordinate> = BTreeSet::new();
        points.insert(YCoordinate::zero());

        for SoundChannel { notes, .. } in &bmson.sound_channels {
            for Note { y, .. } in notes {
                points.insert(pulses_to_y(y.0));
            }
        }

        for MineChannel { notes, .. } in &bmson.mine_channels {
            for MineEvent { y, .. } in notes {
                points.insert(pulses_to_y(y.0));
            }
        }

        for KeyChannel { notes, .. } in &bmson.key_channels {
            for KeyEvent { y, .. } in notes {
                points.insert(pulses_to_y(y.0));
            }
        }

        for ev in &bmson.bpm_events {
            points.insert(pulses_to_y(ev.y.0));
        }

        for ScrollEvent { y, .. } in &bmson.scroll_events {
            points.insert(pulses_to_y(y.0));
        }

        for stop in &bmson.stop_events {
            points.insert(pulses_to_y(stop.y.0));
        }

        for BgaEvent { y, .. } in &bmson.bga.bga_events {
            points.insert(pulses_to_y(y.0));
        }

        for BgaEvent { y, .. } in &bmson.bga.layer_events {
            points.insert(pulses_to_y(y.0));
        }

        for BgaEvent { y, .. } in &bmson.bga.poor_events {
            points.insert(pulses_to_y(y.0));
        }

        if let Some(lines) = &bmson.lines {
            for bar_line in lines {
                points.insert(pulses_to_y(bar_line.y.0));
            }
        } else {
            let max_y = points
                .iter()
                .cloned()
                .max()
                .unwrap_or_else(YCoordinate::zero);
            let floor = max_y.value().to_i64().unwrap_or(0);
            for i in 0..=floor {
                points.insert(YCoordinate::new(Decimal::from(i)));
            }
        }

        // Build BPM map and calculate cumulative times
        let init_bpm: Decimal = bmson.info.init_bpm.as_f64().into();
        let mut bpm_map: BTreeMap<YCoordinate, Decimal> = BTreeMap::new();
        bpm_map.insert(YCoordinate::zero(), init_bpm.clone());

        for ev in &bmson.bpm_events {
            bpm_map.insert(pulses_to_y(ev.y.0), ev.bpm.as_f64().into());
        }

        let mut stop_list: Vec<(YCoordinate, u64)> = bmson
            .stop_events
            .iter()
            .map(|st| (pulses_to_y(st.y.0), st.duration))
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

        let nanos_for_stop = |stop_y: &YCoordinate, stop_pulses: u64| -> u64 {
            let bpm_at_stop = bpm_map
                .range((
                    std::ops::Bound::Unbounded,
                    std::ops::Bound::Included(stop_y),
                ))
                .next_back()
                .map(|(_, b)| b.clone())
                .unwrap_or_else(|| init_bpm.clone());

            if bpm_at_stop > Decimal::zero() {
                let stop_y_len = pulses_to_y(stop_pulses);
                let numerator =
                    stop_y_len.value() * Decimal::from(240u64) * Decimal::from(NANOS_PER_SECOND);
                (numerator / bpm_at_stop).round().to_u64().unwrap_or(0)
            } else {
                0
            }
        };

        let mut stop_idx = 0usize;

        for curr in points {
            if curr <= prev {
                continue;
            }

            let delta_y = Decimal::from(&curr - &prev);
            let delta_nanos = if cur_bpm > Decimal::zero() {
                let numerator = delta_y * Decimal::from(240u64) * Decimal::from(NANOS_PER_SECOND);
                (numerator / cur_bpm).round().to_u64().unwrap_or(0)
            } else {
                0
            };

            total_nanos = total_nanos.saturating_add(delta_nanos);

            while let Some((sy, stop_pulses)) = stop_list.get(stop_idx) {
                if sy > &curr {
                    break;
                }
                if sy > &prev {
                    total_nanos = total_nanos.saturating_add(nanos_for_stop(sy, *stop_pulses));
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

        // Build events map with activate times
        let mut events_map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        let to_time_span = |nanos: u64| TimeSpan::from_duration(Duration::from_nanos(nanos));
        let mut id_gen: ChartEventIdGenerator = ChartEventIdGenerator::default();

        // Sound channel notes
        for SoundChannel { name, notes } in &bmson.sound_channels {
            let mut last_restart_y = YCoordinate::zero();
            for Note { y, x, l, c, .. } in notes {
                let y_coord = pulses_to_y(y.0);
                let wav_id = audio_name_to_id.get(name.as_ref()).copied();

                if let Some((side, key)) = lane_from_x(Some(bmson.info.mode_hint.as_ref()), *x) {
                    let length = (*l > 0).then(|| {
                        let end_y = pulses_to_y(y.0 + l);
                        &end_y - &y_coord
                    });
                    let kind = if *l > 0 {
                        NoteKind::Long
                    } else {
                        NoteKind::Visible
                    };
                    let continue_play = c.then(|| {
                        let to = cum_map.get(&y_coord).copied().unwrap_or(0);
                        let from = cum_map.get(&last_restart_y).copied().unwrap_or(0);
                        to_time_span(to.saturating_sub(from))
                    });

                    let event = crate::chart_process::ChartEvent::Note {
                        side,
                        key,
                        kind,
                        wav_id,
                        length,
                        continue_play,
                    };

                    let at = to_time_span(cum_map.get(&y_coord).copied().unwrap_or(0));
                    let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);

                    if !*c {
                        last_restart_y = y_coord.clone();
                    }

                    events_map.entry(y_coord).or_default().push(evp);
                } else {
                    let event = crate::chart_process::ChartEvent::Bgm { wav_id };
                    let at = to_time_span(cum_map.get(&y_coord).copied().unwrap_or(0));
                    let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                    events_map.entry(y_coord).or_default().push(evp);
                }
            }
        }

        // BPM events
        for ev in &bmson.bpm_events {
            let y = pulses_to_y(ev.y.0);
            let event = crate::chart_process::ChartEvent::BpmChange {
                bpm: ev.bpm.as_f64().into(),
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }

        // Scroll events
        for ScrollEvent { y, rate } in &bmson.scroll_events {
            let y = pulses_to_y(y.0);
            let event = crate::chart_process::ChartEvent::ScrollChange {
                factor: rate.as_f64().into(),
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }

        // BGA events
        let mut id_to_bmp: HashMap<u32, Option<BmpId>> = HashMap::new();
        for BgaHeader { id, name } in &bmson.bga.bga_header {
            id_to_bmp.insert(id.0, bmp_name_to_id.get(name.as_ref()).copied());
        }

        for BgaEvent { y, id } in &bmson.bga.bga_events {
            let y = pulses_to_y(y.0);
            let bmp_id = id_to_bmp.get(&id.0).copied().flatten();
            let event = crate::chart_process::ChartEvent::BgaChange {
                layer: BgaLayer::Base,
                bmp_id,
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }

        for BgaEvent { y, id } in &bmson.bga.layer_events {
            let y = pulses_to_y(y.0);
            let bmp_id = id_to_bmp.get(&id.0).copied().flatten();
            let event = crate::chart_process::ChartEvent::BgaChange {
                layer: BgaLayer::Overlay,
                bmp_id,
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }

        for BgaEvent { y, id } in &bmson.bga.poor_events {
            let y = pulses_to_y(y.0);
            let bmp_id = id_to_bmp.get(&id.0).copied().flatten();
            let event = crate::chart_process::ChartEvent::BgaChange {
                layer: BgaLayer::Poor,
                bmp_id,
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }

        // Bar lines
        if let Some(lines) = &bmson.lines {
            for bar_line in lines {
                let y = pulses_to_y(bar_line.y.0);
                let event = crate::chart_process::ChartEvent::BarLine;
                let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
                let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
                events_map.entry(y).or_default().push(evp);
            }
        } else {
            let max_y = events_map
                .keys()
                .map(super::types::YCoordinate::value)
                .max()
                .cloned()
                .unwrap_or_else(Decimal::zero);

            if max_y > Decimal::zero() {
                let mut current_y = Decimal::zero();
                while current_y <= max_y {
                    let y_coord = YCoordinate::from(current_y.clone());
                    let event = crate::chart_process::ChartEvent::BarLine;
                    let at = to_time_span(cum_map.get(&y_coord).copied().unwrap_or(0));
                    let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                    events_map.entry(y_coord).or_default().push(evp);
                    current_y += Decimal::one();
                }
            }
        }

        // Stop events
        for stop in &bmson.stop_events {
            let y = pulses_to_y(stop.y.0);
            let event = crate::chart_process::ChartEvent::Stop {
                duration: (stop.duration as f64).into(),
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }

        // Mine channel notes
        for MineChannel { name, notes } in &bmson.mine_channels {
            for MineEvent { x, y, .. } in notes {
                let y_coord = pulses_to_y(y.0);
                let Some((side, key)) = lane_from_x(Some(bmson.info.mode_hint.as_ref()), *x) else {
                    continue;
                };
                let wav_id = audio_name_to_id.get(name.as_ref()).copied();
                let event = crate::chart_process::ChartEvent::Note {
                    side,
                    key,
                    kind: NoteKind::Landmine,
                    wav_id,
                    length: None,
                    continue_play: None,
                };
                let at = to_time_span(cum_map.get(&y_coord).copied().unwrap_or(0));
                let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                events_map.entry(y_coord).or_default().push(evp);
            }
        }

        // Key channel notes
        for KeyChannel { name, notes } in &bmson.key_channels {
            for KeyEvent { x, y } in notes {
                let y_coord = pulses_to_y(y.0);
                let Some((side, key)) = lane_from_x(Some(bmson.info.mode_hint.as_ref()), *x) else {
                    continue;
                };
                let wav_id = audio_name_to_id.get(name.as_ref()).copied();
                let event = crate::chart_process::ChartEvent::Note {
                    side,
                    key,
                    kind: NoteKind::Invisible,
                    wav_id,
                    length: None,
                    continue_play: None,
                };
                let at = to_time_span(cum_map.get(&y_coord).copied().unwrap_or(0));
                let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                events_map.entry(y_coord).or_default().push(evp);
            }
        }

        AllEventsIndex::new(events_map)
    }

    /// Build flow events mapping by Y coordinate.
    fn build_flow_events(bmson: &Bmson<'a>) -> BTreeMap<YCoordinate, Vec<FlowEvent>> {
        let mut flow_events_by_y: BTreeMap<YCoordinate, Vec<FlowEvent>> = BTreeMap::new();

        let pulses_denom = Decimal::from(4 * bmson.info.resolution.get());
        let pulses_to_y =
            |pulses: i64| YCoordinate::new(Decimal::from(pulses) / pulses_denom.clone());

        for ev in &bmson.bpm_events {
            let y = pulses_to_y(ev.y.0 as i64);
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Bpm(ev.bpm.as_f64().into()));
        }

        for ScrollEvent { y, rate } in &bmson.scroll_events {
            let y_coord = pulses_to_y(y.0 as i64);
            flow_events_by_y
                .entry(y_coord)
                .or_default()
                .push(FlowEvent::Scroll(rate.as_f64().into()));
        }

        flow_events_by_y
    }
}

/// Helper function to determine lane from x coordinate in BMSON.
#[cfg(feature = "bmson")]
fn lane_from_x(
    mode_hint: Option<&str>,
    x: Option<std::num::NonZeroU8>,
) -> Option<(PlayerSide, Key)> {
    let lane_value = x?.get();

    if !mode_hint
        .map(|hint| hint.starts_with_ignore_case("beat"))
        .unwrap_or(false)
    {
        return Some((PlayerSide::Player1, Key::Key(lane_value)));
    }

    let (adjusted_lane, side) = if lane_value > 8 {
        (lane_value - 8, PlayerSide::Player2)
    } else {
        (lane_value, PlayerSide::Player1)
    };

    let key = match adjusted_lane {
        1..=7 => Key::Key(adjusted_lane),
        8 => Key::Scratch(1),
        _ => return None,
    };

    Some((side, key))
}
