//! Bmson Processor Module.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::time::SystemTime;

use crate::bms::prelude::*;
use crate::bmson::prelude::*;
use crate::chart_process::{
    ChartEvent, ChartEventWithPosition, ChartProcessor, ControlEvent, VisibleEvent,
    types::{BmpId, DisplayRatio, WavId, YCoordinate},
};
use std::str::FromStr;

/// ChartProcessor of Bmson files.
pub struct BmsonProcessor<'a> {
    bmson: Bmson<'a>,

    // Resource ID mappings
    /// Audio filename to WavId mapping
    audio_name_to_id: HashMap<String, WavId>,
    /// Image filename to BmpId mapping
    bmp_name_to_id: HashMap<String, BmpId>,

    // Playback state
    started_at: Option<SystemTime>,
    last_poll_at: Option<SystemTime>,
    progressed_y: Decimal,

    // Flow parameters
    default_visible_y_length: YCoordinate,
    current_bpm: Decimal,
    current_scroll: Decimal,

    /// Pending external events queue
    inbox: Vec<ControlEvent>,

    /// Preloaded events list (all events in current visible area)
    preloaded_events: Vec<ChartEventWithPosition>,

    /// Preprocessed all events mapping, sorted by y coordinate
    all_events: BTreeMap<YCoordinate, Vec<ChartEvent>>,
}

impl<'a> BmsonProcessor<'a> {
    /// Create BMSON processor and initialize playback state with default parameters.
    #[must_use]
    pub fn new(bmson: Bmson<'a>) -> Self {
        let init_bpm: Decimal = bmson.info.init_bpm.as_f64().into();

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

        // Calculate visible Y length based on starting BPM and 600ms reaction time
        // Formula: visible Y length = (BPM / 120.0) * 0.6 seconds
        // Where 0.6 seconds = 600ms, 120.0 is the base BPM
        let reaction_time_seconds = Decimal::from_str("0.6").unwrap(); // 600ms
        let base_bpm = Decimal::from(120);
        let visible_y_length = (init_bpm.clone() / base_bpm) * reaction_time_seconds;

        let mut processor = Self {
            bmson,
            audio_name_to_id,
            bmp_name_to_id,
            started_at: None,
            last_poll_at: None,
            progressed_y: Decimal::from(0),
            inbox: Vec::new(),
            preloaded_events: Vec::new(),
            all_events: BTreeMap::new(),
            default_visible_y_length: YCoordinate::from(visible_y_length),
            current_bpm: init_bpm,
            current_scroll: Decimal::from(1),
        };

        processor.preprocess_events();
        processor
    }

    /// Preprocess all events, create event mapping sorted by y coordinate
    fn preprocess_events(&mut self) {
        let mut events_map: BTreeMap<YCoordinate, Vec<ChartEvent>> = BTreeMap::new();

        // Process sound channel events
        for SoundChannel { name, notes } in &self.bmson.sound_channels {
            for Note { y, x, l, c, .. } in notes {
                let yy = self.pulses_to_y(y.0);
                let y_coord = YCoordinate::from(yy.clone());

                let Some((side, key)) = Self::lane_from_x(x.as_ref().copied()) else {
                    let wav_id = self.get_wav_id_for_name(name);
                    let event = ChartEvent::Bgm { wav_id };
                    events_map.entry(y_coord).or_default().push(event);
                    continue;
                };
                let wav_id = self.get_wav_id_for_name(name);
                let length = (*l > 0).then(|| {
                    let end_y = self.pulses_to_y(y.0 + l);
                    YCoordinate::from(end_y - yy.clone())
                });
                let kind = if *l > 0 {
                    NoteKind::Long
                } else {
                    NoteKind::Visible
                };
                let event = ChartEvent::Note {
                    side,
                    key,
                    kind,
                    wav_id,
                    length,
                    continue_play: *c,
                };
                events_map.entry(y_coord).or_default().push(event);
            }
        }

        // Process BPM events
        for ev in &self.bmson.bpm_events {
            let y = self.pulses_to_y(ev.y.0);
            let y_coord = YCoordinate::from(y);
            let event = ChartEvent::BpmChange {
                bpm: ev.bpm.as_f64().into(),
            };
            events_map.entry(y_coord).or_default().push(event);
        }

        // Process Scroll events
        for ScrollEvent { y, rate } in &self.bmson.scroll_events {
            let y = self.pulses_to_y(y.0);
            let y_coord = YCoordinate::from(y);
            let event = ChartEvent::ScrollChange {
                factor: rate.as_f64().into(),
            };
            events_map.entry(y_coord).or_default().push(event);
        }

        // Process Stop events
        for stop in &self.bmson.stop_events {
            let y = self.pulses_to_y(stop.y.0);
            let y_coord = YCoordinate::from(y);
            let event = ChartEvent::Stop {
                duration: (stop.duration as f64).into(),
            };
            events_map.entry(y_coord).or_default().push(event);
        }

        // Process BGA base layer events
        for BgaEvent { y, id, .. } in &self.bmson.bga.bga_events {
            let yy = self.pulses_to_y(y.0);
            let y_coord = YCoordinate::from(yy);
            let bmp_name = self
                .bmson
                .bga
                .bga_header
                .iter()
                .find(|header| header.id.0 == id.0)
                .map(|header| &*header.name);
            let bmp_id = bmp_name.and_then(|name| self.get_bmp_id_for_name(name));
            let event = ChartEvent::BgaChange {
                layer: BgaLayer::Base,
                bmp_id,
            };
            events_map.entry(y_coord).or_default().push(event);
        }

        // Process BGA overlay layer events
        for BgaEvent { y, id, .. } in &self.bmson.bga.layer_events {
            let yy = self.pulses_to_y(y.0);
            let y_coord = YCoordinate::from(yy);
            let bmp_name = self
                .bmson
                .bga
                .bga_header
                .iter()
                .find(|header| header.id.0 == id.0)
                .map(|header| &*header.name);
            let bmp_id = bmp_name.and_then(|name| self.get_bmp_id_for_name(name));
            let event = ChartEvent::BgaChange {
                layer: BgaLayer::Overlay,
                bmp_id,
            };
            events_map.entry(y_coord).or_default().push(event);
        }

        // Process BGA poor layer events
        for BgaEvent { y, id, .. } in &self.bmson.bga.poor_events {
            let yy = self.pulses_to_y(y.0);
            let y_coord = YCoordinate::from(yy);
            let bmp_name = self
                .bmson
                .bga
                .bga_header
                .iter()
                .find(|header| header.id.0 == id.0)
                .map(|header| &*header.name);
            let bmp_id = bmp_name.and_then(|name| self.get_bmp_id_for_name(name));
            let event = ChartEvent::BgaChange {
                layer: BgaLayer::Poor,
                bmp_id,
            };
            events_map.entry(y_coord).or_default().push(event);
        }

        // Process bar line events - generated last but not exceeding other objects
        if let Some(lines) = &self.bmson.lines {
            for bar_line in lines {
                let y = self.pulses_to_y(bar_line.y.0);
                let y_coord = YCoordinate::from(y);
                let event = ChartEvent::BarLine;
                events_map.entry(y_coord).or_default().push(event);
            }
        } else {
            // If barline is not defined, generate measure lines at each unit Y value, but not exceeding other objects' Y values
            self.generate_auto_barlines(&mut events_map);
        }

        // Process mine channel events
        for MineChannel { name, notes } in &self.bmson.mine_channels {
            for MineEvent { x, y, .. } in notes {
                let yy = self.pulses_to_y(y.0);
                let y_coord = YCoordinate::from(yy);
                let Some((side, key)) = Self::lane_from_x(*x) else {
                    continue;
                };
                let wav_id = self.get_wav_id_for_name(name);
                let event = ChartEvent::Note {
                    side,
                    key,
                    kind: NoteKind::Landmine,
                    wav_id,
                    length: None,
                    continue_play: false,
                };
                events_map.entry(y_coord).or_default().push(event);
            }
        }

        // Process hidden key channel events
        for KeyChannel { name, notes } in &self.bmson.key_channels {
            for KeyEvent { x, y, .. } in notes {
                let yy = self.pulses_to_y(y.0);
                let y_coord = YCoordinate::from(yy);
                let Some((side, key)) = Self::lane_from_x(*x) else {
                    continue;
                };
                let wav_id = self.get_wav_id_for_name(name);
                let event = ChartEvent::Note {
                    side,
                    key,
                    kind: NoteKind::Invisible,
                    wav_id,
                    length: None,
                    continue_play: false,
                };
                events_map.entry(y_coord).or_default().push(event);
            }
        }

        self.all_events = events_map;
    }

    /// Convert pulse count to unified y coordinate (unit: measure). One measure = 4*resolution pulses.
    fn pulses_to_y(&self, pulses: u64) -> Decimal {
        let denom = Decimal::from(4 * self.bmson.info.resolution.get());
        if denom == Decimal::from(0) {
            Decimal::from(0)
        } else {
            Decimal::from(pulses) / denom
        }
    }

    /// Automatically generate measure lines for BMSON without defined barline (at each unit Y value, but not exceeding other objects' Y values)
    fn generate_auto_barlines(&self, events_map: &mut BTreeMap<YCoordinate, Vec<ChartEvent>>) {
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

        // Generate measure lines at each unit Y value, but not exceeding maximum Y value
        let mut current_y = Decimal::from(0);
        while current_y <= max_y {
            let y_coord = YCoordinate::from(current_y.clone());
            let event = ChartEvent::BarLine;
            events_map.entry(y_coord).or_default().push(event);
            current_y += Decimal::from(1);
        }
    }

    /// Get WavId for audio filename
    fn get_wav_id_for_name(&self, name: &str) -> Option<WavId> {
        self.audio_name_to_id.get(name).copied()
    }

    /// Get BmpId for image filename
    fn get_bmp_id_for_name(&self, name: &str) -> Option<BmpId> {
        self.bmp_name_to_id.get(name).copied()
    }

    /// Current instantaneous displacement velocity (y units per second).
    /// y is the normalized measure unit: `y = pulses / (4*resolution)`, one measure equals 1 in default 4/4.
    /// Model: v = (current_bpm / 120.0) (using fixed base BPM 120)
    /// Note: BPM only affects y progression speed, does not change event positions; Scroll only affects display positions.
    fn current_velocity(&self) -> Decimal {
        let base_bpm = Decimal::from(120);
        if self.current_bpm.is_sign_negative() {
            Decimal::from(0)
        } else {
            (self.current_bpm.clone() / base_bpm).max(Decimal::from(0))
        }
    }

    /// Get the next event that affects speed (sorted by y ascending): BPM/SCROLL.
    fn next_flow_event_after(&self, y_from_exclusive: Decimal) -> Option<(Decimal, FlowEvent)> {
        let mut best: Option<(Decimal, FlowEvent)> = None;

        for ev in &self.bmson.bpm_events {
            let y = self.pulses_to_y(ev.y.0);
            if y > y_from_exclusive {
                best = min_by_y_decimal(best, (y, FlowEvent::Bpm(ev.bpm.as_f64().into())));
            }
        }
        for ScrollEvent { y, rate } in &self.bmson.scroll_events {
            let y = self.pulses_to_y(y.0);
            if y > y_from_exclusive {
                best = min_by_y_decimal(best, (y, FlowEvent::Scroll(rate.as_f64().into())));
            }
        }
        best
    }

    fn step_to(&mut self, now: SystemTime) {
        let Some(started) = self.started_at else {
            return;
        };
        let last = self.last_poll_at.unwrap_or(started);
        if now.duration_since(last).unwrap_or_default().is_zero() {
            return;
        }

        let mut remaining_secs =
            Decimal::from(now.duration_since(last).unwrap_or_default().as_secs_f64());
        let mut cur_vel = self.current_velocity();
        let mut cur_y = self.progressed_y.clone();
        loop {
            let next_event = self.next_flow_event_after(cur_y.clone());
            if next_event.is_none()
                || cur_vel == Decimal::from(0)
                || remaining_secs == Decimal::from(0)
            {
                cur_y += cur_vel * remaining_secs;
                break;
            }
            let (event_y, evt) = next_event.unwrap();
            if event_y.clone() <= cur_y.clone() {
                self.apply_flow_event(evt);
                cur_vel = self.current_velocity();
                continue;
            }
            let distance = event_y.clone() - cur_y.clone();
            if cur_vel > Decimal::from(0) {
                let time_to_event_secs = distance / cur_vel.clone();
                if time_to_event_secs <= remaining_secs {
                    cur_y = event_y;
                    remaining_secs -= time_to_event_secs;
                    self.apply_flow_event(evt);
                    cur_vel = self.current_velocity();
                    continue;
                }
            }
            cur_y += cur_vel * remaining_secs;
            break;
        }

        self.progressed_y = cur_y;
        self.last_poll_at = Some(now);
    }

    fn apply_flow_event(&mut self, evt: FlowEvent) {
        match evt {
            FlowEvent::Bpm(bpm) => self.current_bpm = Decimal::from(bpm),
            FlowEvent::Scroll(s) => self.current_scroll = Decimal::from(s),
        }
    }

    fn visible_window_y(&self) -> Decimal {
        // Dynamically calculate visible window length based on current BPM and 600ms reaction time
        // Formula: visible Y length = (current BPM / 120.0) * 0.6 seconds
        let reaction_time_seconds = Decimal::from_str("0.6").unwrap(); // 600ms
        let base_bpm = Decimal::from(120);
        (self.current_bpm.clone() / base_bpm) * reaction_time_seconds
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

impl<'a> ChartProcessor for BmsonProcessor<'a> {
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

    fn default_visible_y_length(&self) -> YCoordinate {
        self.default_visible_y_length.clone()
    }

    fn current_bpm(&self) -> Decimal {
        self.current_bpm.clone()
    }
    fn current_speed(&self) -> Decimal {
        Decimal::from(1)
    }
    fn current_scroll(&self) -> Decimal {
        self.current_scroll.clone()
    }

    fn start_play(&mut self, now: SystemTime) {
        self.started_at = Some(now);
        self.last_poll_at = Some(now);
        self.progressed_y = Decimal::from(0);
        self.preloaded_events.clear();
        self.current_bpm = self.bmson.info.init_bpm.as_f64().into();
    }

    fn update(&mut self, now: SystemTime) -> impl Iterator<Item = ChartEventWithPosition> {
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

        // Get events from preprocessed event mapping
        for (y_coord, events) in &self.all_events {
            let y_value = y_coord.value();

            // Check if it's an event triggered at current moment
            if *y_value > prev_y && *y_value <= cur_y {
                for event in events {
                    triggered_events
                        .push(ChartEventWithPosition::new(y_coord.clone(), event.clone()));
                }
            }

            // Check if it's an event within preload range
            if *y_value > cur_y && *y_value <= preload_end_y {
                for event in events {
                    new_preloaded_events
                        .push(ChartEventWithPosition::new(y_coord.clone(), event.clone()));
                }
            }
        }

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
    Scroll(Decimal),
}

fn min_by_y_decimal(
    best: Option<(Decimal, FlowEvent)>,
    candidate: (Decimal, FlowEvent),
) -> Option<(Decimal, FlowEvent)> {
    match best {
        None => Some(candidate),
        Some((y, _)) if candidate.0 < y => Some(candidate),
        Some(o) => Some(o),
    }
}
