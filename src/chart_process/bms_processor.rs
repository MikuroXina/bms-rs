//! Bms Processor Module.

use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroU64;
use std::path::Path;
use std::time::SystemTime;

use crate::bms::prelude::*;
use crate::chart_process::{
    ChartEvent, ChartEventWithPosition, ChartProcessor, ControlEvent, VisibleEvent,
    types::{BmpId, DisplayRatio, WavId, YCoordinate},
};
use std::str::FromStr;

/// ChartProcessor of Bms files.
pub struct BmsProcessor<T = KeyLayoutBeat>
where
    T: KeyLayoutMapper,
{
    bms: Bms<T>,

    // Playback state
    started_at: Option<SystemTime>,
    last_poll_at: Option<SystemTime>,
    /// Accumulated displacement progressed (y, actual movement distance unit)
    progressed_y: Decimal,

    /// Pending external events queue
    inbox: Vec<ControlEvent>,

    /// All events mapping (sorted by Y coordinate)
    all_events: BTreeMap<YCoordinate, Vec<ChartEvent>>,

    /// Preloaded events list (all events in current visible area)
    preloaded_events: Vec<ChartEventWithPosition>,

    // Flow parameters
    default_visible_y_length: YCoordinate,
    current_bpm: Decimal,
    current_speed: Decimal,
    current_scroll: Decimal,
}

impl<T> BmsProcessor<T>
where
    T: KeyLayoutMapper,
{
    /// Create processor, initialize default parameters
    #[must_use]
    pub fn new(bms: Bms<T>) -> Self {
        // Initialize BPM: prefer chart initial BPM, otherwise 120
        let init_bpm = bms
            .arrangers
            .bpm
            .as_ref()
            .cloned()
            .unwrap_or(Decimal::from(120));

        // Calculate visible Y length based on starting BPM and 600ms reaction time
        // Formula: visible Y length = (BPM / 120.0) * 0.6 seconds
        // Where 0.6 seconds = 600ms, 120.0 is the base BPM
        let reaction_time_seconds = Decimal::from_str("0.6").unwrap(); // 600ms
        let base_bpm = Decimal::from(120);
        let visible_y_length = (init_bpm.clone() / base_bpm) * reaction_time_seconds;

        let all_events = Self::precompute_all_events(&bms);

        Self {
            bms,
            started_at: None,
            last_poll_at: None,
            progressed_y: Decimal::from(0),
            inbox: Vec::new(),
            all_events,
            preloaded_events: Vec::new(),
            default_visible_y_length: YCoordinate::from(visible_y_length),
            current_bpm: init_bpm,
            current_speed: Decimal::from(1),
            current_scroll: Decimal::from(1),
        }
    }

    /// Precompute all events, store grouped by Y coordinate
    /// Note: Speed effects are calculated into event positions during initialization, ensuring event trigger times remain unchanged
    fn precompute_all_events(bms: &Bms<T>) -> BTreeMap<YCoordinate, Vec<ChartEvent>> {
        let mut events_map: BTreeMap<YCoordinate, Vec<ChartEvent>> = BTreeMap::new();

        // Note / Wav arrival events
        for obj in bms.notes().all_notes() {
            let y = Self::y_of_time_static(bms, obj.offset, &bms.arrangers.speed_factor_changes);
            let event = Self::event_for_note_static(bms, obj, y.clone());

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push(event);
        }

        // BPM change events
        for change in bms.arrangers.bpm_changes.values() {
            let y = Self::y_of_time_static(bms, change.time, &bms.arrangers.speed_factor_changes);
            let event = ChartEvent::BpmChange {
                bpm: change.bpm.clone(),
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push(event);
        }

        // Scroll change events
        for change in bms.arrangers.scrolling_factor_changes.values() {
            let y = Self::y_of_time_static(bms, change.time, &bms.arrangers.speed_factor_changes);
            let event = ChartEvent::ScrollChange {
                factor: change.factor.clone(),
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push(event);
        }

        // Speed change events
        for change in bms.arrangers.speed_factor_changes.values() {
            let y = Self::y_of_time_static(bms, change.time, &bms.arrangers.speed_factor_changes);
            let event = ChartEvent::SpeedChange {
                factor: change.factor.clone(),
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push(event);
        }

        // Stop events
        for stop in bms.arrangers.stops.values() {
            let y = Self::y_of_time_static(bms, stop.time, &bms.arrangers.speed_factor_changes);
            let event = ChartEvent::Stop {
                duration: stop.duration.clone(),
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push(event);
        }

        // BGA change events
        for bga_obj in bms.graphics.bga_changes.values() {
            let y = Self::y_of_time_static(bms, bga_obj.time, &bms.arrangers.speed_factor_changes);
            let bmp_index = bga_obj.id.as_u16() as usize;
            let event = ChartEvent::BgaChange {
                layer: bga_obj.layer,
                bmp_id: Some(BmpId::from(bmp_index)),
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push(event);
        }

        // BGA opacity change events (requires minor-command feature)
        #[cfg(feature = "minor-command")]
        for (layer, opacity_changes) in &bms.graphics.bga_opacity_changes {
            for opacity_obj in opacity_changes.values() {
                let y = Self::y_of_time_static(
                    bms,
                    opacity_obj.time,
                    &bms.arrangers.speed_factor_changes,
                );
                let event = ChartEvent::BgaOpacityChange {
                    layer: *layer,
                    opacity: opacity_obj.opacity,
                };

                events_map
                    .entry(YCoordinate::from(y))
                    .or_default()
                    .push(event);
            }
        }

        // BGA ARGB color change events (requires minor-command feature)
        #[cfg(feature = "minor-command")]
        for (layer, argb_changes) in &bms.graphics.bga_argb_changes {
            for argb_obj in argb_changes.values() {
                let y =
                    Self::y_of_time_static(bms, argb_obj.time, &bms.arrangers.speed_factor_changes);
                let argb = ((argb_obj.argb.alpha as u32) << 24)
                    | ((argb_obj.argb.red as u32) << 16)
                    | ((argb_obj.argb.green as u32) << 8)
                    | (argb_obj.argb.blue as u32);
                let event = ChartEvent::BgaArgbChange {
                    layer: *layer,
                    argb,
                };

                events_map
                    .entry(YCoordinate::from(y))
                    .or_default()
                    .push(event);
            }
        }

        // BGM volume change events
        for bgm_volume_obj in bms.notes.bgm_volume_changes.values() {
            let y = Self::y_of_time_static(
                bms,
                bgm_volume_obj.time,
                &bms.arrangers.speed_factor_changes,
            );
            let event = ChartEvent::BgmVolumeChange {
                volume: bgm_volume_obj.volume,
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push(event);
        }

        // KEY volume change events
        for key_volume_obj in bms.notes.key_volume_changes.values() {
            let y = Self::y_of_time_static(
                bms,
                key_volume_obj.time,
                &bms.arrangers.speed_factor_changes,
            );
            let event = ChartEvent::KeyVolumeChange {
                volume: key_volume_obj.volume,
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push(event);
        }

        // Text display events
        for text_obj in bms.notes.text_events.values() {
            let y = Self::y_of_time_static(bms, text_obj.time, &bms.arrangers.speed_factor_changes);
            let event = ChartEvent::TextDisplay {
                text: text_obj.text.clone(),
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push(event);
        }

        // Judge level change events
        for judge_obj in bms.notes.judge_events.values() {
            let y =
                Self::y_of_time_static(bms, judge_obj.time, &bms.arrangers.speed_factor_changes);
            let event = ChartEvent::JudgeLevelChange {
                level: judge_obj.judge_level,
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push(event);
        }

        // Minor-command feature events
        #[cfg(feature = "minor-command")]
        {
            // Video seek events
            for seek_obj in bms.notes.seek_events.values() {
                let y =
                    Self::y_of_time_static(bms, seek_obj.time, &bms.arrangers.speed_factor_changes);
                let event = ChartEvent::VideoSeek {
                    seek_time: seek_obj.position.to_string().parse::<f64>().unwrap_or(0.0),
                };

                events_map
                    .entry(YCoordinate::from(y))
                    .or_default()
                    .push(event);
            }

            // BGA key binding events
            for bga_keybound_obj in bms.notes.bga_keybound_events.values() {
                let y = Self::y_of_time_static(
                    bms,
                    bga_keybound_obj.time,
                    &bms.arrangers.speed_factor_changes,
                );
                let event = ChartEvent::BgaKeybound {
                    event: bga_keybound_obj.event.clone(),
                };

                events_map
                    .entry(YCoordinate::from(y))
                    .or_default()
                    .push(event);
            }

            // Option change events
            for option_obj in bms.notes.option_events.values() {
                let y = Self::y_of_time_static(
                    bms,
                    option_obj.time,
                    &bms.arrangers.speed_factor_changes,
                );
                let event = ChartEvent::OptionChange {
                    option: option_obj.option.clone(),
                };

                events_map
                    .entry(YCoordinate::from(y))
                    .or_default()
                    .push(event);
            }
        }

        // Generate measure lines - generated last but not exceeding other objects
        Self::generate_barlines_for_bms(bms, &mut events_map);

        events_map
    }

    /// Generate measure lines for BMS (generated for each track, but not exceeding other objects' Y values)
    fn generate_barlines_for_bms(
        bms: &Bms<T>,
        events_map: &mut BTreeMap<YCoordinate, Vec<ChartEvent>>,
    ) {
        // Find the maximum Y value of all events
        let max_y = events_map
            .keys()
            .map(|y_coord| y_coord.value())
            .max()
            .cloned()
            .unwrap_or(Decimal::from(0));

        if max_y <= Decimal::from(0) {
            return;
        }

        // Get the track number of the last object
        let last_obj_time = bms.last_obj_time().unwrap_or_else(|| {
            ObjTime::new(
                0,
                0,
                NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
            )
        });

        // Generate measure lines for each track, but not exceeding maximum Y value
        for track in 0..=last_obj_time.track().0 {
            let track_y = Self::y_of_time_static(
                bms,
                ObjTime::new(
                    track,
                    0,
                    NonZeroU64::new(1).expect("1 should be a valid NonZeroU64"),
                ),
                &bms.arrangers.speed_factor_changes,
            );

            if track_y <= max_y {
                let y_coord = YCoordinate::from(track_y);
                let event = ChartEvent::BarLine;
                events_map.entry(y_coord).or_default().push(event);
            }
        }
    }

    /// Static version of y_of_time, considers Speed effects (used for event position precomputation)
    /// Speed effects are calculated into event positions during initialization, ensuring event trigger times remain unchanged
    fn y_of_time_static(
        bms: &Bms<T>,
        time: ObjTime,
        speed_changes: &std::collections::BTreeMap<ObjTime, crate::bms::model::obj::SpeedObj>,
    ) -> Decimal {
        let mut y = Decimal::from(0);
        // Accumulate complete measures
        for t in 0..time.track().0 {
            let section_len = bms
                .arrangers
                .section_len_changes
                .get(&Track(t))
                .map(|s| &s.length)
                .cloned()
                .unwrap_or(Decimal::from(1));
            y += section_len;
        }
        // Accumulate proportionally within current measure
        let current_len = bms
            .arrangers
            .section_len_changes
            .get(&time.track())
            .map(|s| &s.length)
            .cloned()
            .unwrap_or(Decimal::from(1));
        if time.denominator().get() > 0 {
            let fraction =
                Decimal::from(time.numerator()) / Decimal::from(time.denominator().get());
            y += current_len * fraction;
        }

        // Find the last Speed change before current time point as the currently effective Speed factor
        let mut current_speed_factor = Decimal::from(1);
        for (change_time, change) in speed_changes {
            if *change_time <= time {
                current_speed_factor = change.factor.clone();
            }
        }

        // Speed affects both y distance and progression speed, but must keep trigger time unchanged
        // y_new = y_base * current_speed_factor
        y * current_speed_factor
    }

    /// Static version of event_for_note, used for precomputation
    fn event_for_note_static(bms: &Bms<T>, obj: &WavObj, y: Decimal) -> ChartEvent {
        if let Some((side, key, kind)) = Self::lane_of_channel_id(obj.channel_id) {
            let wav_id = Some(WavId::from(obj.wav_id.as_u16() as usize));
            let length = if kind == NoteKind::Long {
                // Long note: find the next note in the same channel to calculate length
                if let Some(next_obj) = bms.notes().next_obj_by_key(obj.channel_id, obj.offset) {
                    let next_y = Self::y_of_time_static(
                        bms,
                        next_obj.offset,
                        &bms.arrangers.speed_factor_changes,
                    );
                    Some(YCoordinate::from(next_y - y.clone()))
                } else {
                    None
                }
            } else {
                None
            };
            ChartEvent::Note {
                side,
                key,
                kind,
                wav_id,
                length,
                continue_play: false, // Fixed as false for BMS
            }
        } else {
            let wav_id = Some(WavId::from(obj.wav_id.as_u16() as usize));
            ChartEvent::Bgm { wav_id }
        }
    }

    /// Get the length of specified Track (SectionLength), default 1.0
    fn section_length_of(&self, track: Track) -> Decimal {
        self.bms
            .arrangers
            .section_len_changes
            .get(&track)
            .map(|s| &s.length)
            .cloned()
            .unwrap_or(Decimal::from(1))
    }

    /// Convert `ObjTime` to cumulative displacement y (unit: measure, default 4/4 one measure equals 1; linearly converted by `#SECLEN`).
    fn y_of_time(&self, time: ObjTime) -> Decimal {
        let mut y = Decimal::from(0);
        // Accumulate complete measures
        for t in 0..time.track().0 {
            y += self.section_length_of(Track(t));
        }
        // Accumulate proportionally within current measure
        let current_len = self.section_length_of(time.track());
        if time.denominator().get() > 0 {
            let fraction =
                Decimal::from(time.numerator()) / Decimal::from(time.denominator().get());
            y += current_len * fraction;
        }
        y
    }

    /// Current instantaneous displacement velocity (y units per second).
    /// Model: v = (current_bpm / 120.0) * speed_factor (using fixed base BPM 120)
    /// Note: Speed affects y progression speed, but does not change actual time progression; Scroll only affects display positions.
    fn current_velocity(&self) -> Decimal {
        let base_bpm = Decimal::from(120);
        if self.current_bpm <= Decimal::from(0) {
            return Decimal::from(0);
        }
        let velocity = self.current_bpm.clone() / base_bpm;
        let speed_factor = self.current_speed.clone();
        let result = velocity * speed_factor;
        result.max(Decimal::from(f64::EPSILON)) // speed must be positive
    }

    /// Get the next event that affects speed (sorted by y ascending): BPM/SCROLL/SPEED changes.
    fn next_flow_event_after(&self, y_from_exclusive: Decimal) -> Option<(Decimal, FlowEvent)> {
        // Collect three event sources, find the minimum item with y greater than threshold
        let mut best: Option<(Decimal, FlowEvent)> = None;

        for change in self.bms.arrangers.bpm_changes.values() {
            let y = self.y_of_time(change.time);
            if y > y_from_exclusive {
                let bpm = change.bpm.clone();
                best = min_by_y_decimal(best, (y, FlowEvent::Bpm(bpm)));
            }
        }
        for change in self.bms.arrangers.scrolling_factor_changes.values() {
            let y = self.y_of_time(change.time);
            if y > y_from_exclusive {
                let factor = change.factor.clone();
                best = min_by_y_decimal(best, (y, FlowEvent::Scroll(factor)));
            }
        }
        for change in self.bms.arrangers.speed_factor_changes.values() {
            let y = self.y_of_time(change.time);
            if y > y_from_exclusive {
                let factor = change.factor.clone();
                best = min_by_y_decimal(best, (y, FlowEvent::Speed(factor)));
            }
        }

        best
    }

    /// Advance time to `now`, perform segmented integration of progress and speed by events.
    fn step_to(&mut self, now: SystemTime) {
        let Some(started) = self.started_at else {
            return;
        };
        let last = self.last_poll_at.unwrap_or(started);
        if now <= last {
            return;
        }

        let mut remaining_secs =
            Decimal::from(now.duration_since(last).unwrap_or_default().as_secs_f64());
        let mut cur_vel = self.current_velocity();
        let mut cur_y = self.progressed_y.clone();
        // Advance in segments until time slice is used up
        loop {
            // The next event that affects speed
            let next_event = self.next_flow_event_after(cur_y.clone());
            if next_event.is_none()
                || cur_vel <= Decimal::from(0)
                || remaining_secs <= Decimal::from(0)
            {
                // Advance directly to the end
                cur_y += cur_vel * remaining_secs;
                break;
            }
            let (event_y, evt) = next_event.unwrap();
            if event_y.clone() <= cur_y.clone() {
                // Defense: avoid infinite loop if event position doesn't advance
                self.apply_flow_event(evt);
                cur_vel = self.current_velocity();
                continue;
            }
            // Time required to reach event
            let distance = event_y.clone() - cur_y.clone();
            if cur_vel > Decimal::from(0) {
                let time_to_event_secs = distance / cur_vel.clone();
                if time_to_event_secs <= remaining_secs {
                    // First advance to event point
                    cur_y = event_y;
                    remaining_secs -= time_to_event_secs;
                    self.apply_flow_event(evt);
                    cur_vel = self.current_velocity();
                    continue;
                }
            }
            // Time not enough to reach event, advance and end
            cur_y += cur_vel * remaining_secs;
            break;
        }

        self.progressed_y = cur_y;
        self.last_poll_at = Some(now);
    }

    fn apply_flow_event(&mut self, evt: FlowEvent) {
        match evt {
            FlowEvent::Bpm(bpm) => self.current_bpm = bpm,
            FlowEvent::Speed(s) => self.current_speed = s,
            FlowEvent::Scroll(s) => self.current_scroll = s,
        }
    }

    /// Calculate visible window length (y units): based on current BPM and 600ms reaction time
    fn visible_window_y(&self) -> Decimal {
        // Dynamically calculate visible window length based on current BPM and 600ms reaction time
        // Formula: visible Y length = (current BPM / 120.0) * 0.6 seconds
        let reaction_time_seconds = Decimal::from_str("0.6").unwrap(); // 600ms
        let base_bpm = Decimal::from(120);
        (self.current_bpm.clone() / base_bpm) * reaction_time_seconds
    }

    fn lane_of_channel_id(channel_id: NoteChannelId) -> Option<(PlayerSide, Key, NoteKind)> {
        if let Some(map) = channel_id.try_into_map::<T>() {
            let side = map.side();
            let key = map.key();
            let kind = map.kind();
            Some((side, key, kind))
        } else {
            None
        }
    }
}

impl<T> ChartProcessor for BmsProcessor<T>
where
    T: KeyLayoutMapper,
{
    fn audio_files(&self) -> HashMap<WavId, &Path> {
        self.bms
            .notes
            .wav_files
            .iter()
            .map(|(obj_id, path)| (WavId::from(obj_id.as_u16() as usize), path.as_path()))
            .collect()
    }

    fn bmp_files(&self) -> HashMap<BmpId, &Path> {
        self.bms
            .graphics
            .bmp_files
            .iter()
            .map(|(obj_id, bmp)| (BmpId::from(obj_id.as_u16() as usize), bmp.file.as_path()))
            .collect()
    }

    fn default_visible_y_length(&self) -> YCoordinate {
        self.default_visible_y_length.clone()
    }

    fn current_bpm(&self) -> Decimal {
        self.current_bpm.clone()
    }
    fn current_speed(&self) -> Decimal {
        self.current_speed.clone()
    }
    fn current_scroll(&self) -> Decimal {
        self.current_scroll.clone()
    }

    fn start_play(&mut self, now: SystemTime) {
        self.started_at = Some(now);
        self.last_poll_at = Some(now);
        self.progressed_y = Decimal::from(0);
        self.preloaded_events.clear();
        // Initialize current_bpm to header or default
        self.current_bpm = self
            .bms
            .arrangers
            .bpm
            .as_ref()
            .cloned()
            .unwrap_or(Decimal::from(120));
    }

    fn update(&mut self, now: SystemTime) -> impl Iterator<Item = ChartEventWithPosition> {
        // Process external events delivered through post_events
        let incoming = std::mem::take(&mut self.inbox);
        for evt in &incoming {
            match evt {
                ControlEvent::SetDefaultVisibleYLength { length } => {
                    self.default_visible_y_length = length.clone();
                }
            }
        }

        let prev_y = self.progressed_y.clone();
        self.step_to(now);
        let cur_y = self.progressed_y.clone();

        // Calculate preload range: current y + visible y range
        let visible_y_length = self.visible_window_y();
        let preload_end_y = cur_y.clone() + visible_y_length;

        // Collect events triggered at current moment
        let mut triggered_events: Vec<ChartEventWithPosition> = Vec::new();

        // Collect events within preload range
        let mut new_preloaded_events: Vec<ChartEventWithPosition> = Vec::new();

        // Get events from precomputed event Map
        for (y_coord, events) in &self.all_events {
            let y = y_coord.value();

            // If event is triggered at current moment
            if *y > prev_y && *y <= cur_y {
                for event in events {
                    triggered_events
                        .push(ChartEventWithPosition::new(y_coord.clone(), event.clone()));
                }
            }

            // If event is within preload range
            if *y > cur_y && *y <= preload_end_y {
                for event in events {
                    new_preloaded_events
                        .push(ChartEventWithPosition::new(y_coord.clone(), event.clone()));
                }
            }
        }

        // Sort events
        triggered_events.sort_by(|a, b| {
            a.position()
                .value()
                .partial_cmp(b.position().value())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        new_preloaded_events.sort_by(|a, b| {
            a.position()
                .value()
                .partial_cmp(b.position().value())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Update preloaded events list
        self.preloaded_events = new_preloaded_events;

        triggered_events.into_iter()
    }

    fn post_events(&mut self, events: &[ControlEvent]) {
        self.inbox.extend_from_slice(events);
    }

    fn visible_events(&mut self, now: SystemTime) -> impl Iterator<Item = VisibleEvent> {
        self.step_to(now);
        let current_y = self.progressed_y.clone();
        let visible_window_y = self.visible_window_y();
        let scroll_factor = self.current_scroll.clone();

        self.preloaded_events.iter().map(move |event_with_pos| {
            let event_y = event_with_pos.position().value();
            // Calculate display ratio: (event_y - current_y) / visible_window_y * scroll_factor
            // Note: scroll can be non-zero positive or negative values
            let display_ratio_value = if visible_window_y > Decimal::from(0) {
                ((event_y.clone() - current_y.clone()) / visible_window_y.clone())
                    * scroll_factor.clone()
            } else {
                Decimal::from(0)
            };
            let display_ratio = DisplayRatio::from(display_ratio_value);
            VisibleEvent::new(
                event_with_pos.position().clone(),
                event_with_pos.event().clone(),
                display_ratio,
            )
        })
    }
}

#[derive(Debug, Clone)]
enum FlowEvent {
    Bpm(Decimal),
    Speed(Decimal),
    Scroll(Decimal),
}

fn min_by_y_decimal(
    best: Option<(Decimal, FlowEvent)>,
    candidate: (Decimal, FlowEvent),
) -> Option<(Decimal, FlowEvent)> {
    match best {
        None => Some(candidate),
        Some((y, _)) if candidate.0 < y => Some(candidate),
        Some(other) => Some(other),
    }
}
