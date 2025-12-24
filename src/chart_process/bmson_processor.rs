//! Bmson Processor Module.
#![cfg(feature = "bmson")]

use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    sync::OnceLock,
    time::Duration,
};

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive, Zero};

use crate::bms::prelude::*;
use crate::bmson::prelude::*;
use crate::chart_process::types::{
    AllEventsIndex, BmpId, ChartEventIdGenerator, DisplayRatio, PlayheadEvent, VisibleChartEvent,
    VisibleRangePerBpm, WavId, YCoordinate,
};
use crate::chart_process::{ChartEvent, ChartProcessor, ControlEvent};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// ChartProcessor of Bmson files.
pub struct BmsonProcessor {
    // Resource ID mappings
    /// Audio filename to WavId mapping
    audio_name_to_id: HashMap<String, WavId>,
    /// Image filename to BmpId mapping
    bmp_name_to_id: HashMap<String, BmpId>,

    // Playback state
    started_at: Option<TimeStamp>,
    last_poll_at: Option<TimeStamp>,
    progressed_y: BigDecimal,

    // Flow parameters
    current_bpm: BigDecimal,
    current_scroll: BigDecimal,
    /// Playback ratio
    playback_ratio: BigDecimal,
    /// Visible range per BPM, representing the relationship between BPM and visible Y range
    visible_range_per_bpm: VisibleRangePerBpm,
    /// Initial BPM at start
    init_bpm: BigDecimal,

    /// Preloaded events list (all events in current visible area)
    preloaded_events: Vec<PlayheadEvent>,

    /// Preprocessed all events mapping, sorted by y coordinate
    all_events: AllEventsIndex,

    /// Indexed flow events by y (BPM/Scroll) for efficient lookup
    flow_events_by_y: BTreeMap<BigDecimal, Vec<FlowEvent>>,
}

impl BmsonProcessor {
    /// Create BMSON processor with visible range per BPM configuration.
    #[must_use]
    pub fn new(bmson: &Bmson<'_>, visible_range_per_bpm: VisibleRangePerBpm) -> Self {
        let init_bpm: BigDecimal = bmson.info.init_bpm.as_f64().into();
        let pulses_denom = BigDecimal::from(4 * bmson.info.resolution.get());
        let pulses_to_y = |pulses: i64| BigDecimal::from(pulses) / pulses_denom.clone();

        // Preprocessing: assign IDs to all audio and image resources
        let mut audio_name_to_id = HashMap::new();
        let mut bmp_name_to_id = HashMap::new();
        let mut next_audio_id = 0usize;
        let mut next_bmp_id = 0usize;

        // Process audio files
        for sound_channel in &bmson.sound_channels {
            let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(sound_channel.name.to_string())
            else {
                continue;
            };
            e.insert(WavId::new(next_audio_id));
            next_audio_id += 1;
        }

        // Process mine audio files
        for mine_channel in &bmson.mine_channels {
            let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(mine_channel.name.to_string())
            else {
                continue;
            };
            e.insert(WavId::new(next_audio_id));
            next_audio_id += 1;
        }

        // Process hidden key audio files
        for key_channel in &bmson.key_channels {
            let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(key_channel.name.to_string())
            else {
                continue;
            };
            e.insert(WavId::new(next_audio_id));
            next_audio_id += 1;
        }

        // Process image files
        for BgaHeader { name, .. } in &bmson.bga.bga_header {
            let std::collections::hash_map::Entry::Vacant(e) =
                bmp_name_to_id.entry(name.to_string())
            else {
                continue;
            };
            e.insert(BmpId::new(next_bmp_id));
            next_bmp_id += 1;
        }

        let playback_ratio = BigDecimal::one();

        // Pre-index flow events by y for fast next_flow_event_after
        let mut flow_events_by_y: BTreeMap<BigDecimal, Vec<FlowEvent>> = BTreeMap::new();
        for ev in &bmson.bpm_events {
            let y = pulses_to_y(ev.y.0 as i64);
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Bpm(ev.bpm.as_f64().into()));
        }
        for ScrollEvent { y, rate } in &bmson.scroll_events {
            let y = pulses_to_y(y.0 as i64);
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Scroll(rate.as_f64().into()));
        }

        let all_events =
            AllEventsIndex::precompute_events(bmson, &audio_name_to_id, &bmp_name_to_id);

        Self {
            audio_name_to_id,
            bmp_name_to_id,
            started_at: None,
            last_poll_at: None,
            progressed_y: BigDecimal::zero(),
            preloaded_events: Vec::new(),
            all_events,
            current_bpm: init_bpm.clone(),
            current_scroll: BigDecimal::one(),
            playback_ratio,
            visible_range_per_bpm,
            flow_events_by_y,
            init_bpm,
        }
    }

    /// Current instantaneous displacement velocity (y units per second).
    /// y is the normalized measure unit: `y = pulses / (4*resolution)`, one measure equals 1 in default 4/4.
    fn current_velocity(&self) -> BigDecimal {
        if self.current_bpm.is_sign_negative() {
            BigDecimal::zero()
        } else {
            let denom = BigDecimal::from(240);
            (self.current_bpm.clone() / denom * self.playback_ratio.clone()).max(BigDecimal::zero())
        }
    }

    /// Get the next event that affects speed (sorted by y ascending): BPM/SCROLL.
    fn next_flow_event_after(
        &self,
        y_from_exclusive: &BigDecimal,
    ) -> Option<(BigDecimal, FlowEvent)> {
        use std::ops::Bound::{Excluded, Unbounded};
        self.flow_events_by_y
            .range((Excluded(y_from_exclusive), Unbounded))
            .next()
            .and_then(|(y, events)| events.first().cloned().map(|evt| (y.clone(), evt)))
    }

    fn step_to(&mut self, now: TimeStamp) {
        let Some(started) = self.started_at else {
            return;
        };
        let last = self.last_poll_at.unwrap_or(started);
        if now <= last {
            return;
        }

        let mut remaining_time = now - last;
        let mut cur_vel = self.current_velocity();
        let mut cur_y = self.progressed_y.clone();
        loop {
            let cur_y_now = cur_y;
            let next_event = self.next_flow_event_after(&cur_y_now);
            if next_event.is_none()
                || cur_vel == BigDecimal::zero()
                || remaining_time <= TimeSpan::ZERO
            {
                cur_y = cur_y_now + cur_vel * remaining_time.as_nanos().max(0) / NANOS_PER_SECOND;
                break;
            }
            let Some((event_y, evt)) = next_event else {
                cur_y = cur_y_now + cur_vel * remaining_time.as_nanos().max(0) / NANOS_PER_SECOND;
                break;
            };
            if event_y <= cur_y_now {
                self.apply_flow_event(evt);
                cur_vel = self.current_velocity();
                cur_y = cur_y_now;
                continue;
            }
            let distance = event_y.clone() - cur_y_now.clone();
            if cur_vel > BigDecimal::zero() {
                let time_to_event_nanos = ((distance / cur_vel.clone())
                    * BigDecimal::from(NANOS_PER_SECOND))
                .to_u64()
                .unwrap_or(0);
                let time_to_event =
                    TimeSpan::from_duration(Duration::from_nanos(time_to_event_nanos));
                if time_to_event <= remaining_time {
                    cur_y = event_y;
                    remaining_time -= time_to_event;
                    self.apply_flow_event(evt);
                    cur_vel = self.current_velocity();
                    continue;
                }
            }
            cur_y = cur_y_now + cur_vel * remaining_time.as_nanos().max(0) / NANOS_PER_SECOND;
            break;
        }

        self.progressed_y = cur_y;
        self.last_poll_at = Some(now);
    }

    fn apply_flow_event(&mut self, evt: FlowEvent) {
        match evt {
            FlowEvent::Bpm(bpm) => self.current_bpm = bpm,
            FlowEvent::Scroll(s) => self.current_scroll = s,
        }
    }

    fn visible_window_y(&self) -> BigDecimal {
        self.visible_range_per_bpm.window_y(&self.current_bpm)
    }

    fn lane_from_x(x: Option<std::num::NonZeroU8>) -> Option<(PlayerSide, Key)> {
        let lane_value = x.map_or(0, |l| l.get());
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
}

impl ChartProcessor for BmsonProcessor {
    fn audio_files(&self) -> HashMap<WavId, &Path> {
        // Note: Audio file paths in BMSON are relative to the chart file, here returning virtual paths
        // When actually used, these paths need to be resolved based on the chart file location
        self.audio_name_to_id
            .iter()
            .map(|(name, id)| (*id, Path::new(name)))
            .collect()
    }

    fn bmp_files(&self) -> HashMap<BmpId, &Path> {
        // Note: Image file paths in BMSON are relative to the chart file, here returning virtual paths
        // When actually used, these paths need to be resolved based on the chart file location
        self.bmp_name_to_id
            .iter()
            .map(|(name, id)| (*id, Path::new(name)))
            .collect()
    }

    fn visible_range_per_bpm(&self) -> &VisibleRangePerBpm {
        &self.visible_range_per_bpm
    }

    fn current_bpm(&self) -> &BigDecimal {
        &self.current_bpm
    }
    fn current_speed(&self) -> &BigDecimal {
        // to avoid repeated allocations
        static DECIMAL_ONE: OnceLock<BigDecimal> = OnceLock::new();
        DECIMAL_ONE.get_or_init(BigDecimal::one)
    }
    fn current_scroll(&self) -> &BigDecimal {
        &self.current_scroll
    }

    fn start_play(&mut self, now: TimeStamp) {
        self.started_at = Some(now);
        self.last_poll_at = Some(now);
        self.progressed_y = BigDecimal::zero();
        self.preloaded_events.clear();
        self.current_bpm = self.init_bpm.clone();
    }

    fn started_at(&self) -> Option<TimeStamp> {
        self.started_at
    }

    fn update(&mut self, now: TimeStamp) -> impl Iterator<Item = PlayheadEvent> {
        let prev_y = self.progressed_y.clone();
        self.step_to(now);
        let cur_y = self.progressed_y.clone();

        // Calculate preload range: current y + visible y range
        let visible_y_length = self.visible_window_y();
        let preload_end_y = cur_y.clone() + visible_y_length;

        use std::ops::Bound::{Excluded, Included};

        // Collect events triggered at current moment
        let triggered_events = self.all_events.events_in_y_range((
            Excluded(YCoordinate::from(prev_y)),
            Included(YCoordinate::from(cur_y.clone())),
        ));

        let new_preloaded_events = self.all_events.events_in_y_range((
            Excluded(YCoordinate::from(cur_y)),
            Included(YCoordinate::from(preload_end_y)),
        ));

        // Update preloaded events list
        self.preloaded_events = new_preloaded_events;

        triggered_events.into_iter()
    }

    fn events_in_time_range(
        &mut self,
        range: impl std::ops::RangeBounds<TimeSpan>,
    ) -> impl Iterator<Item = PlayheadEvent> {
        let events: Vec<PlayheadEvent> = if let Some(started) = self.started_at {
            let last = self.last_poll_at.unwrap_or(started);
            // Calculate center time: elapsed time scaled by playback ratio
            let elapsed = last
                .checked_elapsed_since(started)
                .unwrap_or(TimeSpan::ZERO);
            let center_nanos = (BigDecimal::from(elapsed.as_nanos().max(0))
                * self.playback_ratio.clone())
            .to_u64()
            .unwrap_or(0);
            let center = TimeSpan::from_duration(Duration::from_nanos(center_nanos));
            self.all_events
                .events_in_time_range_offset_from(center, range)
        } else {
            Vec::new()
        };

        events.into_iter()
    }

    fn post_events(&mut self, events: impl Iterator<Item = ControlEvent>) {
        for evt in events {
            match evt {
                ControlEvent::SetVisibleRangePerBpm {
                    visible_range_per_bpm,
                } => {
                    self.visible_range_per_bpm = visible_range_per_bpm;
                }
                ControlEvent::SetPlaybackRatio { ratio } => {
                    self.playback_ratio = ratio;
                }
            }
        }
    }

    fn visible_events(&mut self, now: TimeStamp) -> impl Iterator<Item = VisibleChartEvent> {
        self.step_to(now);
        let current_y = self.progressed_y.clone();
        let visible_window_y = self.visible_window_y();
        let scroll_factor = self.current_scroll.clone();

        self.preloaded_events.iter().map(move |event_with_pos| {
            let event_y = event_with_pos.position().value();
            // Calculate display ratio: (event_y - current_y) / visible_window_y * scroll_factor
            // Note: scroll can be non-zero positive or negative values
            let display_ratio_value = if visible_window_y > BigDecimal::zero() {
                ((event_y.clone() - current_y.clone()) / visible_window_y.clone())
                    * scroll_factor.clone()
            } else {
                BigDecimal::zero()
            };
            let display_ratio = DisplayRatio::from(display_ratio_value);

            let activate_time = event_with_pos.activate_time;

            VisibleChartEvent::new(
                event_with_pos.id,
                event_with_pos.position().clone(),
                event_with_pos.event().clone(),
                display_ratio,
                activate_time,
            )
        })
    }

    fn playback_ratio(&self) -> &BigDecimal {
        &self.playback_ratio
    }
}

#[derive(Debug, Clone)]
enum FlowEvent {
    Bpm(BigDecimal),
    Scroll(BigDecimal),
}

impl AllEventsIndex {
    fn precompute_events<'a>(
        bmson: &Bmson<'a>,
        audio_name_to_id: &HashMap<String, WavId>,
        bmp_name_to_id: &HashMap<String, BmpId>,
    ) -> Self {
        use std::collections::BTreeSet;
        let denom = BigDecimal::from(4 * bmson.info.resolution.get());
        let denom_inv = if denom == BigDecimal::zero() {
            BigDecimal::zero()
        } else {
            BigDecimal::one() / denom
        };
        let pulses_to_y = |pulses: u64| BigDecimal::from(pulses) * denom_inv.clone();
        let mut points: BTreeSet<BigDecimal> = BTreeSet::new();
        points.insert(BigDecimal::zero());
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
                .unwrap_or_else(BigDecimal::zero);
            let floor = max_y.to_i64().unwrap_or(0);
            for i in 0..=floor {
                points.insert(BigDecimal::from(i));
            }
        }
        let init_bpm: BigDecimal = bmson.info.init_bpm.as_f64().into();
        let mut bpm_map: BTreeMap<BigDecimal, BigDecimal> = BTreeMap::new();
        bpm_map.insert(BigDecimal::zero(), init_bpm.clone());
        for ev in &bmson.bpm_events {
            bpm_map.insert(pulses_to_y(ev.y.0), ev.bpm.as_f64().into());
        }
        let mut stop_list: Vec<(BigDecimal, u64)> = bmson
            .stop_events
            .iter()
            .map(|st| (pulses_to_y(st.y.0), st.duration))
            .collect();
        stop_list.sort_by(|a, b| a.0.cmp(&b.0));
        let mut cum_map: BTreeMap<BigDecimal, u64> = BTreeMap::new();
        let mut total_nanos: u64 = 0;
        let mut prev = BigDecimal::zero();
        cum_map.insert(prev.clone(), 0);
        let mut cur_bpm = bpm_map
            .range((std::ops::Bound::Unbounded, std::ops::Bound::Included(&prev)))
            .next_back()
            .map(|(_, b)| b.clone())
            .unwrap_or_else(|| init_bpm.clone());
        let nanos_for_stop = |stop_y: &BigDecimal, stop_pulses: u64| {
            let bpm_at_stop = bpm_map
                .range((
                    std::ops::Bound::Unbounded,
                    std::ops::Bound::Included(stop_y),
                ))
                .next_back()
                .map(|(_, b)| b.clone())
                .unwrap_or_else(|| init_bpm.clone());
            let stop_y_len = pulses_to_y(stop_pulses);
            let stop_y_len_f64 = stop_y_len.to_f64().unwrap_or(0.0);
            let bpm_at_stop_f64 = bpm_at_stop.to_f64().unwrap_or(120.0);
            let stop_nanos_f64 = stop_y_len_f64 * 240.0 / bpm_at_stop_f64 * NANOS_PER_SECOND as f64;
            if stop_nanos_f64.is_finite() && stop_nanos_f64 > 0.0 {
                stop_nanos_f64.round() as u64
            } else {
                0
            }
        };
        let mut stop_idx = 0usize;
        for curr in points.into_iter() {
            if curr <= prev {
                continue;
            }
            let delta_y_f64 = (curr.clone() - prev.clone()).to_f64().unwrap_or(0.0);
            let cur_bpm_f64 = cur_bpm.to_f64().unwrap_or(120.0);
            let delta_nanos_f64 = delta_y_f64 * 240.0 / cur_bpm_f64 * NANOS_PER_SECOND as f64;
            if delta_nanos_f64.is_finite() && delta_nanos_f64 > 0.0 {
                total_nanos = total_nanos.saturating_add(delta_nanos_f64.round() as u64);
            }
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
        let mut events_map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        let to_time_span = |nanos: u64| TimeSpan::from_duration(Duration::from_nanos(nanos));
        let mut id_gen: ChartEventIdGenerator = ChartEventIdGenerator::default();
        for SoundChannel { name, notes } in &bmson.sound_channels {
            let mut last_restart_y = BigDecimal::zero();
            for Note { y, x, l, c, .. } in notes {
                let y_coord = YCoordinate::from(pulses_to_y(y.0));
                let wav_id = audio_name_to_id.get(name.as_ref()).copied();
                if let Some((side, key)) = BmsonProcessor::lane_from_x(x.as_ref().copied()) {
                    let length = (*l > 0).then(|| {
                        let end_y = pulses_to_y(y.0 + l);
                        YCoordinate::from(end_y - y_coord.value().clone())
                    });
                    let kind = if *l > 0 {
                        NoteKind::Long
                    } else {
                        NoteKind::Visible
                    };
                    let continue_play = c.then(|| {
                        let to = cum_map.get(y_coord.value()).copied().unwrap_or(0);
                        let from = cum_map.get(&last_restart_y).copied().unwrap_or(0);
                        to_time_span(to.saturating_sub(from))
                    });
                    let event = ChartEvent::Note {
                        side,
                        key,
                        kind,
                        wav_id,
                        length,
                        continue_play,
                    };
                    let at = to_time_span(cum_map.get(y_coord.value()).copied().unwrap_or(0));
                    let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                    if !*c {
                        last_restart_y = y_coord.value().clone();
                    }
                    events_map.entry(y_coord).or_default().push(evp);
                } else {
                    let event = ChartEvent::Bgm { wav_id };
                    let at = to_time_span(cum_map.get(y_coord.value()).copied().unwrap_or(0));
                    let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                    events_map.entry(y_coord).or_default().push(evp);
                }
            }
        }
        for ev in &bmson.bpm_events {
            let y = pulses_to_y(ev.y.0);
            let y_coord = YCoordinate::from(y);
            let event = ChartEvent::BpmChange {
                bpm: ev.bpm.as_f64().into(),
            };
            let at = to_time_span(cum_map.get(y_coord.value()).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
            events_map.entry(y_coord).or_default().push(evp);
        }
        for ScrollEvent { y, rate } in &bmson.scroll_events {
            let y = pulses_to_y(y.0);
            let y_coord = YCoordinate::from(y);
            let event = ChartEvent::ScrollChange {
                factor: rate.as_f64().into(),
            };
            let at = to_time_span(cum_map.get(y_coord.value()).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
            events_map.entry(y_coord).or_default().push(evp);
        }
        let mut id_to_bmp: HashMap<u32, Option<BmpId>> = HashMap::new();
        for BgaHeader { id, name, .. } in &bmson.bga.bga_header {
            id_to_bmp.insert(id.0, bmp_name_to_id.get(name.as_ref()).copied());
        }
        for BgaEvent { y, id, .. } in &bmson.bga.bga_events {
            let yy = pulses_to_y(y.0);
            let y_coord = YCoordinate::from(yy);
            let bmp_id = id_to_bmp.get(&id.0).copied().flatten();
            let event = ChartEvent::BgaChange {
                layer: BgaLayer::Base,
                bmp_id,
            };
            let at = to_time_span(cum_map.get(y_coord.value()).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
            events_map.entry(y_coord).or_default().push(evp);
        }
        for BgaEvent { y, id, .. } in &bmson.bga.layer_events {
            let yy = pulses_to_y(y.0);
            let y_coord = YCoordinate::from(yy);
            let bmp_id = id_to_bmp.get(&id.0).copied().flatten();
            let event = ChartEvent::BgaChange {
                layer: BgaLayer::Overlay,
                bmp_id,
            };
            let at = to_time_span(cum_map.get(y_coord.value()).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
            events_map.entry(y_coord).or_default().push(evp);
        }
        for BgaEvent { y, id, .. } in &bmson.bga.poor_events {
            let yy = pulses_to_y(y.0);
            let y_coord = YCoordinate::from(yy);
            let bmp_id = id_to_bmp.get(&id.0).copied().flatten();
            let event = ChartEvent::BgaChange {
                layer: BgaLayer::Poor,
                bmp_id,
            };
            let at = to_time_span(cum_map.get(y_coord.value()).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
            events_map.entry(y_coord).or_default().push(evp);
        }
        if let Some(lines) = &bmson.lines {
            for bar_line in lines {
                let y = pulses_to_y(bar_line.y.0);
                let y_coord = YCoordinate::from(y);
                let event = ChartEvent::BarLine;
                let at = to_time_span(cum_map.get(y_coord.value()).copied().unwrap_or(0));
                let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                events_map.entry(y_coord).or_default().push(evp);
            }
        } else {
            let max_y = events_map
                .keys()
                .map(|y_coord| y_coord.value())
                .max()
                .cloned()
                .unwrap_or_else(BigDecimal::zero);
            if max_y > BigDecimal::zero() {
                let mut current_y = BigDecimal::zero();
                while current_y <= max_y {
                    let y_coord = YCoordinate::from(current_y.clone());
                    let event = ChartEvent::BarLine;
                    let at = to_time_span(cum_map.get(y_coord.value()).copied().unwrap_or(0));
                    let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                    events_map.entry(y_coord).or_default().push(evp);
                    current_y += BigDecimal::one();
                }
            }
        }
        for stop in &bmson.stop_events {
            let y = pulses_to_y(stop.y.0);
            let y_coord = YCoordinate::from(y);
            let event = ChartEvent::Stop {
                duration: (stop.duration as f64).into(),
            };
            let at = to_time_span(cum_map.get(y_coord.value()).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
            events_map.entry(y_coord).or_default().push(evp);
        }
        for MineChannel { name, notes } in &bmson.mine_channels {
            for MineEvent { x, y, .. } in notes {
                let y_coord = YCoordinate::from(pulses_to_y(y.0));
                let Some((side, key)) = BmsonProcessor::lane_from_x(*x) else {
                    continue;
                };
                let wav_id = audio_name_to_id.get(name.as_ref()).copied();
                let event = ChartEvent::Note {
                    side,
                    key,
                    kind: NoteKind::Landmine,
                    wav_id,
                    length: None,
                    continue_play: None,
                };
                let at = to_time_span(cum_map.get(y_coord.value()).copied().unwrap_or(0));
                let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                events_map.entry(y_coord).or_default().push(evp);
            }
        }
        for KeyChannel { name, notes } in &bmson.key_channels {
            for KeyEvent { x, y, .. } in notes {
                let y_coord = YCoordinate::from(pulses_to_y(y.0));
                let Some((side, key)) = BmsonProcessor::lane_from_x(*x) else {
                    continue;
                };
                let wav_id = audio_name_to_id.get(name.as_ref()).copied();
                let event = ChartEvent::Note {
                    side,
                    key,
                    kind: NoteKind::Invisible,
                    wav_id,
                    length: None,
                    continue_play: None,
                };
                let at = to_time_span(cum_map.get(y_coord.value()).copied().unwrap_or(0));
                let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                events_map.entry(y_coord).or_default().push(evp);
            }
        }
        Self::new(events_map)
    }
}
