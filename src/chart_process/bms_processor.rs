//! Bms Processor Module.

use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroU64;
use std::path::Path;
use std::time::SystemTime;

use crate::bms::prelude::*;
use crate::chart_process::utils::{compute_default_visible_y_length, compute_visible_window_y};
use crate::chart_process::{
    ChartEvent, ChartEventWithPosition, ChartProcessor, ControlEvent, VisibleEvent,
    types::{
        BaseBpmGenerateStyle, BmpId, ChartEventId, ChartEventIdGenerator, DisplayRatio, WavId,
        YCoordinate,
    },
};

/// ChartProcessor of Bms files.
pub struct BmsProcessor {
    bms: Bms,

    // Playback state
    started_at: Option<SystemTime>,
    last_poll_at: Option<SystemTime>,
    /// Accumulated displacement progressed (y, actual movement distance unit)
    progressed_y: Decimal,

    /// Pending external events queue
    inbox: Vec<ControlEvent>,

    /// All events mapping (sorted by Y coordinate)
    all_events: BTreeMap<YCoordinate, Vec<(ChartEventId, ChartEvent)>>,

    /// Preloaded events list (all events in current visible area)
    preloaded_events: Vec<ChartEventWithPosition>,

    // Flow parameters
    default_visible_y_length: YCoordinate,
    current_bpm: Decimal,
    current_speed: Decimal,
    current_scroll: Decimal,
    /// Selected base BPM used for velocity and visible window calculations
    base_bpm: Decimal,
    /// Reaction time in seconds used to derive visible window length
    reaction_time_seconds: Decimal,

    /// Indexed flow events by y (for fast lookup of next flow-affecting event)
    flow_events_by_y: BTreeMap<Decimal, Vec<FlowEvent>>,
}

impl BmsProcessor {
    /// Create processor with explicit reaction time configuration, initialize default parameters
    #[must_use]
    pub fn new<T: KeyLayoutMapper>(
        bms: Bms,
        base_bpm_style: BaseBpmGenerateStyle,
        reaction_time_seconds: Decimal,
    ) -> Self {
        // Initialize BPM: prefer chart initial BPM, otherwise 120
        let init_bpm = bms
            .bpm
            .bpm
            .as_ref()
            .cloned()
            .unwrap_or_else(|| Decimal::from(120));

        // Compute base BPM for visible window based on user-selected style
        let base_bpm = Self::select_base_bpm_for_bms(&bms, base_bpm_style);
        // Compute default visible y length via shared helper
        let default_visible_y_length =
            compute_default_visible_y_length(base_bpm.clone(), reaction_time_seconds.clone());

        let all_events = Self::precompute_all_events::<T>(&bms);

        // Pre-index flow events by y for fast next_flow_event_after
        let mut flow_events_by_y: BTreeMap<Decimal, Vec<FlowEvent>> = BTreeMap::new();
        for change in bms.bpm.bpm_changes.values() {
            let y = {
                // y_of_time only considers section length, matching original next_flow_event_after semantics
                let mut y = Decimal::from(0);
                // Accumulate complete measures
                for t in 0..change.time.track().0 {
                    y += bms
                        .section_len
                        .section_len_changes
                        .get(&Track(t))
                        .map(|s| s.length.clone())
                        .unwrap_or_else(|| Decimal::from(1));
                }
                // Accumulate proportionally within current measure
                let current_len = bms
                    .section_len
                    .section_len_changes
                    .get(&change.time.track())
                    .map(|s| s.length.clone())
                    .unwrap_or_else(|| Decimal::from(1));
                let fraction = if change.time.denominator().get() > 0 {
                    Decimal::from(change.time.numerator())
                        / Decimal::from(change.time.denominator().get())
                } else {
                    Default::default()
                };
                y + current_len * fraction
            };
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Bpm(change.bpm.clone()));
        }
        for change in bms.scroll.scrolling_factor_changes.values() {
            let y = {
                let mut y = Decimal::from(0);
                for t in 0..change.time.track().0 {
                    y += bms
                        .section_len
                        .section_len_changes
                        .get(&Track(t))
                        .map(|s| s.length.clone())
                        .unwrap_or_else(|| Decimal::from(1));
                }
                let current_len = bms
                    .section_len
                    .section_len_changes
                    .get(&change.time.track())
                    .map(|s| s.length.clone())
                    .unwrap_or_else(|| Decimal::from(1));
                let fraction = if change.time.denominator().get() > 0 {
                    Decimal::from(change.time.numerator())
                        / Decimal::from(change.time.denominator().get())
                } else {
                    Default::default()
                };
                y + current_len * fraction
            };
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Scroll(change.factor.clone()));
        }
        for change in bms.speed.speed_factor_changes.values() {
            let y = {
                let mut y = Decimal::from(0);
                for t in 0..change.time.track().0 {
                    y += bms
                        .section_len
                        .section_len_changes
                        .get(&Track(t))
                        .map(|s| s.length.clone())
                        .unwrap_or_else(|| Decimal::from(1));
                }
                let current_len = bms
                    .section_len
                    .section_len_changes
                    .get(&change.time.track())
                    .map(|s| s.length.clone())
                    .unwrap_or_else(|| Decimal::from(1));
                let fraction = if change.time.denominator().get() > 0 {
                    Decimal::from(change.time.numerator())
                        / Decimal::from(change.time.denominator().get())
                } else {
                    Default::default()
                };
                y + current_len * fraction
            };
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Speed(change.factor.clone()));
        }

        Self {
            bms,
            started_at: None,
            last_poll_at: None,
            progressed_y: Decimal::from(0),
            inbox: Vec::new(),
            all_events,
            preloaded_events: Vec::new(),
            default_visible_y_length,
            current_bpm: init_bpm,
            current_speed: Decimal::from(1),
            current_scroll: Decimal::from(1),
            base_bpm,
            reaction_time_seconds,
            flow_events_by_y,
        }
    }

    /// Select a base BPM from a BMS chart according to style.
    fn select_base_bpm_for_bms(bms: &Bms, style: BaseBpmGenerateStyle) -> Decimal {
        match style {
            BaseBpmGenerateStyle::Manual(v) => v,
            BaseBpmGenerateStyle::StartBpm => bms
                .bpm
                .bpm
                .as_ref()
                .cloned()
                .unwrap_or_else(|| Decimal::from(120)),
            BaseBpmGenerateStyle::MinBpm => {
                let mut min: Option<Decimal> = bms.bpm.bpm.as_ref().cloned();
                for change in bms.bpm.bpm_changes.values() {
                    min = match min {
                        Some(curr) => Some(if change.bpm < curr {
                            change.bpm.clone()
                        } else {
                            curr
                        }),
                        None => Some(change.bpm.clone()),
                    };
                }
                min.unwrap_or_else(|| Decimal::from(120))
            }
            BaseBpmGenerateStyle::MaxBpm => {
                let mut max: Option<Decimal> = bms.bpm.bpm.as_ref().cloned();
                for change in bms.bpm.bpm_changes.values() {
                    max = match max {
                        Some(curr) => Some(if change.bpm > curr {
                            change.bpm.clone()
                        } else {
                            curr
                        }),
                        None => Some(change.bpm.clone()),
                    };
                }
                max.unwrap_or_else(|| Decimal::from(120))
            }
        }
    }

    /// Precompute all events, store grouped by Y coordinate
    /// Note: Speed effects are calculated into event positions during initialization, ensuring event trigger times remain unchanged
    fn precompute_all_events<T: KeyLayoutMapper>(
        bms: &Bms,
    ) -> BTreeMap<YCoordinate, Vec<(ChartEventId, ChartEvent)>> {
        let mut events_map: BTreeMap<YCoordinate, Vec<(ChartEventId, ChartEvent)>> =
            BTreeMap::new();
        let mut id_gen: ChartEventIdGenerator = ChartEventIdGenerator::default();

        // Note / Wav arrival events
        for obj in bms.notes().all_notes() {
            let y = Self::y_of_time_static(bms, obj.offset, &bms.speed.speed_factor_changes);
            let event = Self::event_for_note_static::<T>(bms, obj, y.clone());

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push((id_gen.next_id(), event));
        }

        // BPM change events
        for change in bms.bpm.bpm_changes.values() {
            let y = Self::y_of_time_static(bms, change.time, &bms.speed.speed_factor_changes);
            let event = ChartEvent::BpmChange {
                bpm: change.bpm.clone(),
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push((id_gen.next_id(), event));
        }

        // Scroll change events
        for change in bms.scroll.scrolling_factor_changes.values() {
            let y = Self::y_of_time_static(bms, change.time, &bms.speed.speed_factor_changes);
            let event = ChartEvent::ScrollChange {
                factor: change.factor.clone(),
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push((id_gen.next_id(), event));
        }

        // Speed change events
        for change in bms.speed.speed_factor_changes.values() {
            let y = Self::y_of_time_static(bms, change.time, &bms.speed.speed_factor_changes);
            let event = ChartEvent::SpeedChange {
                factor: change.factor.clone(),
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push((id_gen.next_id(), event));
        }

        // Stop events
        for stop in bms.stop.stops.values() {
            let y = Self::y_of_time_static(bms, stop.time, &bms.speed.speed_factor_changes);
            let event = ChartEvent::Stop {
                duration: stop.duration.clone(),
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push((id_gen.next_id(), event));
        }

        // BGA change events
        for bga_obj in bms.bmp.bga_changes.values() {
            let y = Self::y_of_time_static(bms, bga_obj.time, &bms.speed.speed_factor_changes);
            let bmp_index = bga_obj.id.as_u16() as usize;
            let event = ChartEvent::BgaChange {
                layer: bga_obj.layer,
                bmp_id: Some(BmpId::from(bmp_index)),
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push((id_gen.next_id(), event));
        }

        // BGA opacity change events (requires minor-command feature)

        for (layer, opacity_changes) in &bms.bmp.bga_opacity_changes {
            for opacity_obj in opacity_changes.values() {
                let y =
                    Self::y_of_time_static(bms, opacity_obj.time, &bms.speed.speed_factor_changes);
                let event = ChartEvent::BgaOpacityChange {
                    layer: *layer,
                    opacity: opacity_obj.opacity,
                };

                events_map
                    .entry(YCoordinate::from(y))
                    .or_default()
                    .push((id_gen.next_id(), event));
            }
        }

        // BGA ARGB color change events (requires minor-command feature)

        for (layer, argb_changes) in &bms.bmp.bga_argb_changes {
            for argb_obj in argb_changes.values() {
                let y = Self::y_of_time_static(bms, argb_obj.time, &bms.speed.speed_factor_changes);
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
                    .push((id_gen.next_id(), event));
            }
        }

        // BGM volume change events
        for bgm_volume_obj in bms.volume.bgm_volume_changes.values() {
            let y =
                Self::y_of_time_static(bms, bgm_volume_obj.time, &bms.speed.speed_factor_changes);
            let event = ChartEvent::BgmVolumeChange {
                volume: bgm_volume_obj.volume,
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push((id_gen.next_id(), event));
        }

        // KEY volume change events
        for key_volume_obj in bms.volume.key_volume_changes.values() {
            let y =
                Self::y_of_time_static(bms, key_volume_obj.time, &bms.speed.speed_factor_changes);
            let event = ChartEvent::KeyVolumeChange {
                volume: key_volume_obj.volume,
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push((id_gen.next_id(), event));
        }

        // Text display events
        for text_obj in bms.text.text_events.values() {
            let y = Self::y_of_time_static(bms, text_obj.time, &bms.speed.speed_factor_changes);
            let event = ChartEvent::TextDisplay {
                text: text_obj.text.clone(),
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push((id_gen.next_id(), event));
        }

        // Judge level change events
        for judge_obj in bms.judge.judge_events.values() {
            let y = Self::y_of_time_static(bms, judge_obj.time, &bms.speed.speed_factor_changes);
            let event = ChartEvent::JudgeLevelChange {
                level: judge_obj.judge_level,
            };

            events_map
                .entry(YCoordinate::from(y))
                .or_default()
                .push((id_gen.next_id(), event));
        }

        // Minor-command feature events

        {
            // Video seek events
            for seek_obj in bms.video.seek_events.values() {
                let y = Self::y_of_time_static(bms, seek_obj.time, &bms.speed.speed_factor_changes);
                let event = ChartEvent::VideoSeek {
                    seek_time: seek_obj.position.to_string().parse::<f64>().unwrap_or(0.0),
                };

                events_map
                    .entry(YCoordinate::from(y))
                    .or_default()
                    .push((id_gen.next_id(), event));
            }

            // BGA key binding events
            for bga_keybound_obj in bms.bmp.bga_keybound_events.values() {
                let y = Self::y_of_time_static(
                    bms,
                    bga_keybound_obj.time,
                    &bms.speed.speed_factor_changes,
                );
                let event = ChartEvent::BgaKeybound {
                    event: bga_keybound_obj.event.clone(),
                };

                events_map
                    .entry(YCoordinate::from(y))
                    .or_default()
                    .push((id_gen.next_id(), event));
            }

            // Option change events
            for option_obj in bms.option.option_events.values() {
                let y =
                    Self::y_of_time_static(bms, option_obj.time, &bms.speed.speed_factor_changes);
                let event = ChartEvent::OptionChange {
                    option: option_obj.option.clone(),
                };

                events_map
                    .entry(YCoordinate::from(y))
                    .or_default()
                    .push((id_gen.next_id(), event));
            }
        }

        // Generate measure lines - generated last but not exceeding other objects
        Self::generate_barlines_for_bms(bms, &mut events_map, &mut id_gen);

        events_map
    }

    /// Generate measure lines for BMS (generated for each track, but not exceeding other objects' Y values)
    fn generate_barlines_for_bms(
        bms: &Bms,
        events_map: &mut BTreeMap<YCoordinate, Vec<(ChartEventId, ChartEvent)>>,
        id_gen: &mut ChartEventIdGenerator,
    ) {
        // Find the maximum Y value of all events
        let max_y = events_map
            .keys()
            .map(|y_coord| y_coord.value())
            .max()
            .cloned()
            .unwrap_or_else(|| Decimal::from(0));

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
                &bms.speed.speed_factor_changes,
            );

            if track_y <= max_y {
                let y_coord = YCoordinate::from(track_y);
                let event = ChartEvent::BarLine;
                events_map
                    .entry(y_coord)
                    .or_default()
                    .push((id_gen.next_id(), event));
            }
        }
    }

    /// Static version of y_of_time, considers Speed effects (used for event position precomputation)
    /// Speed effects are calculated into event positions during initialization, ensuring event trigger times remain unchanged
    fn y_of_time_static(
        bms: &Bms,
        time: ObjTime,
        speed_changes: &std::collections::BTreeMap<ObjTime, crate::bms::model::obj::SpeedObj>,
    ) -> Decimal {
        let mut y = Decimal::from(0);
        // Accumulate complete measures
        for t in 0..time.track().0 {
            let section_len = bms
                .section_len
                .section_len_changes
                .get(&Track(t))
                .map(|s| &s.length)
                .cloned()
                .unwrap_or_else(|| Decimal::from(1));
            y += section_len;
        }
        // Accumulate proportionally within current measure
        let current_len = bms
            .section_len
            .section_len_changes
            .get(&time.track())
            .map(|s| &s.length)
            .cloned()
            .unwrap_or_else(|| Decimal::from(1));
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
    fn event_for_note_static<T: KeyLayoutMapper>(
        bms: &Bms,
        obj: &WavObj,
        y: Decimal,
    ) -> ChartEvent {
        let Some((side, key, kind)) = Self::lane_of_channel_id::<T>(obj.channel_id) else {
            let wav_id = Some(WavId::from(obj.wav_id.as_u16() as usize));
            return ChartEvent::Bgm { wav_id };
        };
        let wav_id = Some(WavId::from(obj.wav_id.as_u16() as usize));
        let length = (kind == NoteKind::Long)
            .then(|| {
                // Long note: find the next note in the same channel to calculate length
                bms.notes()
                    .next_obj_by_key(obj.channel_id, obj.offset)
                    .map(|next_obj| {
                        let next_y = Self::y_of_time_static(
                            bms,
                            next_obj.offset,
                            &bms.speed.speed_factor_changes,
                        );
                        YCoordinate::from(next_y - y.clone())
                    })
            })
            .flatten();
        ChartEvent::Note {
            side,
            key,
            kind,
            wav_id,
            length,
            continue_play: false, // Fixed as false for BMS
        }
    }

    /// Current instantaneous displacement velocity (y units per second).
    /// Model: v = (current_bpm / base_bpm) * speed_factor
    /// Note: Speed affects y progression speed, but does not change actual time progression; Scroll only affects display positions.
    fn current_velocity(&self) -> Decimal {
        let velocity = if self.current_bpm > Decimal::from(0) {
            let velocity = self.current_bpm.clone() / self.base_bpm.clone();
            let speed_factor = self.current_speed.clone();
            velocity * speed_factor
        } else {
            Default::default()
        };
        velocity.max(Decimal::from(f64::EPSILON))
    }

    /// Get the next event that affects speed (sorted by y ascending): BPM/SCROLL/SPEED changes.
    fn next_flow_event_after(&self, y_from_exclusive: Decimal) -> Option<(Decimal, FlowEvent)> {
        use std::ops::Bound::{Excluded, Unbounded};
        self.flow_events_by_y
            .range((Excluded(y_from_exclusive), Unbounded))
            .next()
            .map(|(y, events)| (y.clone(), events[0].clone()))
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

    /// Calculate visible window length (y units): based on current BPM, base BPM and configured reaction time
    fn visible_window_y(&self) -> Decimal {
        compute_visible_window_y(
            self.current_bpm.clone(),
            self.base_bpm.clone(),
            self.reaction_time_seconds.clone(),
        )
    }

    fn lane_of_channel_id<T: KeyLayoutMapper>(
        channel_id: NoteChannelId,
    ) -> Option<(PlayerSide, Key, NoteKind)> {
        let map = channel_id.try_into_map::<T>()?;
        let side = map.side();
        let key = map.key();
        let kind = map.kind();
        Some((side, key, kind))
    }
}

impl ChartProcessor for BmsProcessor {
    fn audio_files(&self) -> HashMap<WavId, &Path> {
        self.bms
            .wav
            .wav_files
            .iter()
            .map(|(obj_id, path)| (WavId::from(obj_id.as_u16() as usize), path.as_path()))
            .collect()
    }

    fn bmp_files(&self) -> HashMap<BmpId, &Path> {
        self.bms
            .bmp
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
            .bpm
            .bpm
            .as_ref()
            .cloned()
            .unwrap_or_else(|| Decimal::from(120));
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

        use std::ops::Bound::{Excluded, Included};
        // Triggered events: (prev_y, cur_y]
        for (y_coord, events) in self.all_events.range((
            Excluded(YCoordinate::from(prev_y.clone())),
            Included(YCoordinate::from(cur_y.clone())),
        )) {
            for (id, event) in events {
                let evp = ChartEventWithPosition::new(*id, y_coord.clone(), event.clone());
                triggered_events.push(evp);
            }
        }

        // Preloaded events: (cur_y, preload_end_y]
        for (y_coord, events) in self.all_events.range((
            Excluded(YCoordinate::from(cur_y.clone())),
            Included(YCoordinate::from(preload_end_y.clone())),
        )) {
            for (id, event) in events {
                let evp = ChartEventWithPosition::new(*id, y_coord.clone(), event.clone());
                new_preloaded_events.push(evp);
            }
        }

        // Sort to maintain stable order if needed (BTreeMap range is ordered by y)
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
                Default::default()
            };
            let display_ratio = DisplayRatio::from(display_ratio_value);

            VisibleEvent::new(
                event_with_pos.id,
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
