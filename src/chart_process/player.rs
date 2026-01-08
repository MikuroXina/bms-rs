//! Chart Player Module.
//!
//! Unified player for parsed charts, managing playback state and event processing.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Duration;

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive, Zero};

use crate::bms::Decimal;
use crate::chart_process::types::{
    AllEventsIndex, BmpId, DisplayRatio, FlowEvent, PlaybackState, PlayheadEvent,
    VisibleRangePerBpm, WavId, YCoordinate,
};
use crate::chart_process::{ChartEvent, ControlEvent};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Unified chart player.
///
/// This player takes a parsed chart and manages all playback state and event processing.
pub struct ChartPlayer {
    // Parsed chart data
    chart: crate::chart_process::types::ParsedChart,

    // Playback state
    started_at: Option<TimeStamp>,
    last_poll_at: Option<TimeStamp>,

    // Configuration
    pub(crate) visible_range_per_bpm: VisibleRangePerBpm,

    // Performance: velocity caching
    cached_velocity: Option<Decimal>,
    velocity_dirty: bool,

    // Event management
    pub(crate) preloaded_events: Vec<PlayheadEvent>,
    pub(crate) all_events: AllEventsIndex,

    // Flow event indexing
    pub(crate) flow_events_by_y: BTreeMap<YCoordinate, Vec<FlowEvent>>,
    pub(crate) init_bpm: Decimal,

    // Playback state (None before start_play is called)
    playback_state: Option<PlaybackState>,
}

impl ChartPlayer {
    /// Create a new player from a parsed chart.
    #[must_use]
    pub fn new(
        mut chart: crate::chart_process::types::ParsedChart,
        visible_range_per_bpm: VisibleRangePerBpm,
    ) -> Self {
        // Extract flow_events and events from chart (take ownership)
        let flow_events = std::mem::take(&mut chart.flow_events);
        let all_events = chart.events.clone();
        let init_bpm = chart.init_bpm.clone();

        Self {
            chart,
            started_at: None,
            last_poll_at: None,
            visible_range_per_bpm,
            cached_velocity: None,
            velocity_dirty: true,
            preloaded_events: Vec::new(),
            all_events,
            flow_events_by_y: flow_events,
            init_bpm,
            playback_state: None,
        }
    }

    // ===== Playback Control =====

    /// Start playback at the given time.
    pub fn start_play(&mut self, now: TimeStamp) {
        self.started_at = Some(now);
        self.last_poll_at = Some(now);
        self.preloaded_events.clear();
        self.mark_velocity_dirty();

        // Create playback state
        self.playback_state = Some(PlaybackState::new(
            self.init_bpm.clone(),
            self.chart.init_speed.clone(),
            Decimal::one(),
            Decimal::one(),
            YCoordinate::zero(),
        ));
    }

    /// Update playback to the given time, return triggered events.
    ///
    /// Returns empty vec if playback has not started.
    pub fn update(&mut self, now: TimeStamp) -> Vec<PlayheadEvent> {
        // Early return if not started
        if self.started_at.is_none() {
            return Vec::new();
        }

        let state = self.playback_state.as_ref().unwrap();
        let prev_y = state.progressed_y.clone();
        let speed = state.current_speed.clone();
        self.step_to(now, &speed);
        let state = self.playback_state.as_ref().unwrap();
        let cur_y = state.progressed_y.clone();

        // Calculate preload range: current y + visible y range
        let visible_y_length = self.visible_window_y(&state.current_speed);
        let preload_end_y = &cur_y + &visible_y_length;

        use std::ops::Bound::{Excluded, Included};

        // Collect events triggered at current moment
        let mut triggered_events = self.events_in_y_range((Excluded(&prev_y), Included(&cur_y)));

        self.update_preloaded_events(&preload_end_y);

        // Apply Speed changes
        for event in &triggered_events {
            if let ChartEvent::SpeedChange { factor } = event.event()
                && let Some(state) = &mut self.playback_state
            {
                state.current_speed = factor.clone();
            }
        }

        // Sort to maintain stable order
        triggered_events.sort_by(|a, b| {
            a.position()
                .value()
                .partial_cmp(b.position().value())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        triggered_events
    }

    /// Post control events to the player.
    pub fn post_events(&mut self, events: impl Iterator<Item = ControlEvent>) {
        for evt in events {
            self.handle_control_event(evt);
        }
    }

    // ===== State Query =====

    /// Get current playback state.
    ///
    /// Returns `None` if `start_play()` has not been called yet.
    /// Returns `Some(state)` if playback has started.
    #[must_use]
    pub const fn playback_state(&self) -> Option<&PlaybackState> {
        self.playback_state.as_ref()
    }

    /// Get audio file resources (id to path mapping).
    #[must_use]
    pub const fn audio_files(&self) -> &HashMap<WavId, PathBuf> {
        self.chart.resources().wav_files()
    }

    /// Get BGA/BMP image resources (id to path mapping).
    #[must_use]
    pub const fn bmp_files(&self) -> &HashMap<BmpId, PathBuf> {
        self.chart.resources().bmp_files()
    }

    /// Get visible range per BPM.
    #[must_use]
    pub const fn visible_range_per_bpm(&self) -> &VisibleRangePerBpm {
        &self.visible_range_per_bpm
    }

    // ===== Visible Events =====

    /// Get all events in current visible area (with display positions).
    ///
    /// Returns `None` if `start_play()` has not been called yet.
    pub fn visible_events(
        &mut self,
    ) -> Option<Vec<(PlayheadEvent, std::ops::RangeInclusive<DisplayRatio>)>> {
        let state = self.playback_state.as_ref()?;

        let current_y = &state.progressed_y;
        let visible_window_y = self.visible_window_y(&state.current_speed);
        let scroll_factor = &state.current_scroll;

        let result = self
            .preloaded_events
            .iter()
            .map(|event_with_pos| {
                let event_y = event_with_pos.position();
                let start_display_ratio = Self::compute_display_ratio(
                    event_y,
                    current_y,
                    &visible_window_y,
                    scroll_factor,
                );

                // Calculate end position for long notes
                let end_display_ratio = if let ChartEvent::Note {
                    length: Some(length),
                    ..
                } = event_with_pos.event()
                {
                    let end_y = event_y.clone() + length.clone();
                    Self::compute_display_ratio(&end_y, current_y, &visible_window_y, scroll_factor)
                } else {
                    // Normal notes and other events: start and end are the same
                    start_display_ratio.clone()
                };

                (
                    event_with_pos.clone(),
                    start_display_ratio..=end_display_ratio,
                )
            })
            .collect::<Vec<_>>();

        Some(result)
    }

    /// Query events in a time window.
    pub fn events_in_time_range(
        &self,
        range: impl std::ops::RangeBounds<TimeSpan>,
    ) -> Vec<PlayheadEvent> {
        self.started_at.map_or_else(Vec::new, |started| {
            let last = self.last_poll_at.unwrap_or(started);
            // Calculate center time: elapsed time scaled by playback ratio
            let elapsed = last
                .checked_elapsed_since(started)
                .unwrap_or(TimeSpan::ZERO);
            let elapsed_nanos = elapsed.as_nanos().max(0) as u64;
            let elapsed_nanos = Decimal::from(elapsed_nanos);
            let playback_ratio = self
                .playback_state
                .as_ref()
                .map_or_else(Decimal::one, |state| state.playback_ratio.clone());
            let center_nanos = (elapsed_nanos * playback_ratio).to_u64().unwrap_or(0);
            let center = TimeSpan::from_duration(Duration::from_nanos(center_nanos));
            self.all_events
                .events_in_time_range_offset_from(center, range)
        })
    }

    // ===== Internal Core Methods =====

    /// Calculate velocity with caching.
    ///
    /// Formula: `velocity = (bpm / 240) * speed * playback_ratio`
    pub fn calculate_velocity(&mut self, speed: &Decimal) -> Decimal {
        if self.velocity_dirty || self.cached_velocity.is_none() {
            let computed = self.compute_velocity(speed);
            self.cached_velocity = Some(computed.clone());
            self.velocity_dirty = false;
            computed
        } else {
            // SAFETY: We know cached_velocity is Some because we checked is_none above
            self.cached_velocity
                .as_ref()
                .expect("cached_velocity should be Some when not dirty")
                .clone()
        }
    }

    /// Compute velocity without caching (internal use).
    fn compute_velocity(&self, speed: &Decimal) -> Decimal {
        let state = self.playback_state.as_ref();
        let current_bpm = state.map_or_else(|| self.init_bpm.clone(), |s| s.current_bpm.clone());
        let playback_ratio = state.map_or_else(Decimal::one, |s| s.playback_ratio.clone());

        if current_bpm <= Decimal::zero() {
            Decimal::from(f64::EPSILON)
        } else {
            let denom = Decimal::from(240);
            let base = &current_bpm / &denom;
            let v1 = base * speed.clone();
            let v = &v1 * &playback_ratio;
            v.max(Decimal::from(f64::EPSILON))
        }
    }

    /// Mark velocity cache as dirty.
    pub const fn mark_velocity_dirty(&mut self) {
        self.velocity_dirty = true;
    }

    /// Get the next flow event after the given Y position (exclusive).
    #[must_use]
    pub fn next_flow_event_after(
        &self,
        y_from_exclusive: &YCoordinate,
    ) -> Option<(YCoordinate, FlowEvent)> {
        use std::ops::Bound::{Excluded, Unbounded};
        self.flow_events_by_y
            .range((Excluded(y_from_exclusive), Unbounded))
            .next()
            .and_then(|(y, events)| events.first().cloned().map(|evt| (y.clone(), evt)))
    }

    /// Get the next flow event Y position after the given Y (exclusive).
    #[must_use]
    fn next_flow_event_y_after(&self, y_from_exclusive: &YCoordinate) -> Option<YCoordinate> {
        use std::ops::Bound::{Excluded, Unbounded};
        self.flow_events_by_y
            .range((Excluded(y_from_exclusive), Unbounded))
            .next()
            .map(|(y, _)| y.clone())
    }

    /// Apply all flow events at the given Y position.
    fn apply_flow_events_at(&mut self, y: &YCoordinate) {
        // Remove events from the map to take ownership, avoiding borrow conflicts
        if let Some(events) = self.flow_events_by_y.remove(y) {
            for event in events {
                self.apply_flow_event(event);
            }
            // Note: events are not re-inserted since they've been applied
        }
    }

    /// Apply a flow event to this player.
    fn apply_flow_event(&mut self, event: FlowEvent) {
        match event {
            FlowEvent::Bpm(bpm) => {
                self.mark_velocity_dirty();
                if let Some(state) = &mut self.playback_state {
                    state.current_bpm = bpm;
                }
            }
            FlowEvent::Speed(_s) => {
                // Speed is format-specific (BMS only)
                // Handled in update() method
            }
            FlowEvent::Scroll(s) => {
                if let Some(state) = &mut self.playback_state {
                    state.current_scroll = s;
                }
                // Scroll doesn't affect velocity
            }
        }
    }

    /// Advance time to `now`, performing segmented integration.
    ///
    /// This is the core time progression algorithm, shared between BMS and BMSON.
    fn step_to(&mut self, now: TimeStamp, speed: &Decimal) {
        let Some(started) = self.started_at else {
            return;
        };
        let last = self.last_poll_at.unwrap_or(started);
        if now <= last {
            return;
        }

        let mut remaining_time = now - last;
        let mut cur_vel = self.calculate_velocity(speed);
        let mut cur_y = self.playback_state.as_ref().unwrap().progressed_y.clone();

        // Advance in segments until time slice is used up
        loop {
            let cur_y_now = cur_y.clone();
            let next_event_y = self.next_flow_event_y_after(&cur_y_now);

            if next_event_y.is_none()
                || cur_vel <= Decimal::zero()
                || remaining_time <= TimeSpan::ZERO
            {
                // Advance directly to the end
                let delta_y = (cur_vel * Decimal::from(remaining_time.as_nanos().max(0)))
                    / Decimal::from(NANOS_PER_SECOND);
                cur_y = cur_y_now + YCoordinate::new(delta_y.round());
                break;
            }

            let Some(event_y) = next_event_y else {
                let delta_y = (cur_vel * Decimal::from(remaining_time.as_nanos().max(0)))
                    / Decimal::from(NANOS_PER_SECOND);
                cur_y = cur_y_now + YCoordinate::new(delta_y.round());
                break;
            };

            if event_y <= cur_y_now {
                // Defense: avoid infinite loop if event position doesn't advance
                // Apply all events at this Y position
                self.apply_flow_events_at(&event_y);
                cur_vel = self.calculate_velocity(speed);
                cur_y = cur_y_now;
                continue;
            }

            // Time required to reach event
            let distance = event_y.clone() - cur_y_now.clone();
            if cur_vel > Decimal::zero() {
                let time_to_event_nanos = ((distance.value() / &cur_vel)
                    * Decimal::from(NANOS_PER_SECOND))
                .round()
                .to_u64()
                .unwrap_or(0);
                let time_to_event =
                    TimeSpan::from_duration(Duration::from_nanos(time_to_event_nanos));

                if time_to_event <= remaining_time {
                    // First advance to event point
                    cur_y = event_y.clone();
                    remaining_time -= time_to_event;
                    // Apply all events at this Y position
                    self.apply_flow_events_at(&event_y);
                    cur_vel = self.calculate_velocity(speed);
                    continue;
                }
            }

            // Time not enough to reach event, advance and end
            cur_y = cur_y_now
                + YCoordinate::new(
                    cur_vel * Decimal::from(remaining_time.as_nanos().max(0)) / NANOS_PER_SECOND,
                );
            break;
        }

        // Update playback state
        if let Some(state) = &mut self.playback_state {
            state.progressed_y = cur_y;
        }
        self.last_poll_at = Some(now);
    }

    /// Get visible window length in Y units.
    #[must_use]
    pub fn visible_window_y(&self, speed: &Decimal) -> YCoordinate {
        let state = self.playback_state.as_ref();
        let current_bpm = state.map_or_else(|| self.init_bpm.clone(), |s| s.current_bpm.clone());
        let playback_ratio = state.map_or_else(Decimal::one, |s| s.playback_ratio.clone());
        self.visible_range_per_bpm
            .window_y(&current_bpm, speed, &playback_ratio)
    }

    /// Get events in a Y range.
    pub fn events_in_y_range<R>(&self, range: R) -> Vec<PlayheadEvent>
    where
        R: Clone + std::ops::RangeBounds<YCoordinate>,
    {
        self.all_events.events_in_y_range(range)
    }

    /// Update preloaded events based on current Y position.
    pub fn update_preloaded_events(&mut self, preload_end_y: &YCoordinate) {
        use std::ops::Bound::{Excluded, Included};

        let state = self
            .playback_state
            .as_ref()
            .expect("playback_state should be Some");
        let cur_y = &state.progressed_y;
        let new_preloaded_events = self
            .all_events
            .events_in_y_range((Excluded(cur_y), Included(preload_end_y)));

        self.preloaded_events = new_preloaded_events;
    }

    /// Get preloaded events.
    #[must_use]
    pub const fn preloaded_events(&self) -> &Vec<PlayheadEvent> {
        &self.preloaded_events
    }

    /// Compute display ratio for an event.
    #[must_use]
    pub fn compute_display_ratio(
        event_y: &YCoordinate,
        current_y: &YCoordinate,
        visible_window_y: &YCoordinate,
        scroll_factor: &Decimal,
    ) -> DisplayRatio {
        let window_value = visible_window_y.value();
        if window_value > &Decimal::zero() {
            let ratio_value = (event_y - current_y).value() / window_value * scroll_factor.clone();
            DisplayRatio::from(ratio_value)
        } else {
            // Should not happen theoretically; indicates configuration issue if it does
            DisplayRatio::at_judgment_line()
        }
    }

    /// Handle control events.
    pub fn handle_control_event(&mut self, event: ControlEvent) {
        match event {
            ControlEvent::SetVisibleRangePerBpm {
                visible_range_per_bpm,
            } => {
                self.visible_range_per_bpm = visible_range_per_bpm;
            }
            ControlEvent::SetPlaybackRatio { ratio } => {
                self.mark_velocity_dirty();
                if let Some(state) = &mut self.playback_state {
                    state.playback_ratio = ratio;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::chart_process::types::{BaseBpm, ChartResources, ParsedChart};

    #[test]
    fn test_velocity_caching() {
        let chart = ParsedChart::new(
            ChartResources::new(HashMap::new(), HashMap::new()),
            AllEventsIndex::new(BTreeMap::new()),
            BTreeMap::new(),
            Decimal::from(120),
            Decimal::one(),
        );

        let mut player = ChartPlayer::new(
            chart,
            VisibleRangePerBpm::new(&BaseBpm::new(Decimal::from(120)), TimeSpan::SECOND),
        );

        let speed = Decimal::one();

        // First call computes velocity
        let v1 = player.calculate_velocity(&speed);
        assert!(v1 > Decimal::zero());

        // Second call should use cache
        let v2 = player.calculate_velocity(&speed);
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_flow_event_application() {
        use std::collections::BTreeMap;

        let y_event = YCoordinate::from(100.0);

        let mut flow_events_by_y = BTreeMap::new();
        flow_events_by_y.insert(
            y_event,
            vec![
                FlowEvent::Bpm(Decimal::from(180)),
                FlowEvent::Scroll(Decimal::from(1.5)),
            ],
        );

        let chart = ParsedChart::new(
            ChartResources::new(HashMap::new(), HashMap::new()),
            AllEventsIndex::new(BTreeMap::new()),
            flow_events_by_y,
            Decimal::from(120),
            Decimal::one(),
        );

        let mut player = ChartPlayer::new(
            chart,
            VisibleRangePerBpm::new(&BaseBpm::new(Decimal::from(120)), TimeSpan::SECOND),
        );

        // Initial state
        assert_eq!(player.init_bpm, Decimal::from(120));
        assert!(player.playback_state.is_none());

        // Apply BPM change (should do nothing without start_play)
        player.apply_flow_event(FlowEvent::Bpm(Decimal::from(180)));

        assert!(player.playback_state.is_none());
    }

    #[test]
    fn test_flow_event_application_after_start() {
        use std::collections::BTreeMap;

        let y_event = YCoordinate::from(100.0);

        let mut flow_events_by_y = BTreeMap::new();
        flow_events_by_y.insert(
            y_event,
            vec![
                FlowEvent::Bpm(Decimal::from(180)),
                FlowEvent::Scroll(Decimal::from(1.5)),
            ],
        );

        let chart = ParsedChart::new(
            ChartResources::new(HashMap::new(), HashMap::new()),
            AllEventsIndex::new(BTreeMap::new()),
            flow_events_by_y,
            Decimal::from(120),
            Decimal::one(),
        );

        let mut player = ChartPlayer::new(
            chart,
            VisibleRangePerBpm::new(&BaseBpm::new(Decimal::from(120)), TimeSpan::SECOND),
        );

        // Start playback first
        player.start_play(TimeStamp::now());

        // Initial state after start_play
        assert_eq!(
            player.playback_state().unwrap().current_bpm(),
            &Decimal::from(120)
        );
        assert_eq!(
            player.playback_state().unwrap().current_scroll(),
            &Decimal::one()
        );

        // Apply BPM change
        player.apply_flow_event(FlowEvent::Bpm(Decimal::from(180)));

        assert_eq!(
            player.playback_state().unwrap().current_bpm(),
            &Decimal::from(180)
        );
        assert!(player.velocity_dirty);
    }

    #[test]
    fn test_display_ratio_computation() {
        let current_y = YCoordinate::from(10.0);
        let event_y = YCoordinate::from(11.0);
        let visible_window_y = YCoordinate::from(2.0);
        let scroll_factor = Decimal::one();

        let ratio = ChartPlayer::compute_display_ratio(
            &event_y,
            &current_y,
            &visible_window_y,
            &scroll_factor,
        );

        // (11 - 10) / 2 = 0.5
        assert!((ratio.value().to_f64().unwrap() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_multiple_flow_events_same_y_all_triggered() {
        use std::collections::BTreeMap;

        // Setup: Create flow events at the same Y position
        let y_event = YCoordinate::from(100.0);

        let mut flow_events_by_y = BTreeMap::new();
        flow_events_by_y.insert(
            y_event,
            vec![
                FlowEvent::Bpm(Decimal::from(180)),
                FlowEvent::Scroll(Decimal::from(1.5)),
            ],
        );

        let chart = ParsedChart::new(
            ChartResources::new(HashMap::new(), HashMap::new()),
            AllEventsIndex::new(BTreeMap::new()),
            flow_events_by_y,
            Decimal::from(120),
            Decimal::one(),
        );

        let mut player = ChartPlayer::new(
            chart,
            VisibleRangePerBpm::new(&BaseBpm::new(Decimal::from(120)), TimeSpan::SECOND),
        );

        // Start playback
        let start_time = TimeStamp::now();
        player.start_play(start_time);

        // Initial state
        assert_eq!(
            player.playback_state().unwrap().current_bpm(),
            &Decimal::from(120)
        );
        assert_eq!(
            player.playback_state().unwrap().current_scroll(),
            &Decimal::one()
        );

        // Advance past the event Y position
        // Calculate time needed: distance / velocity
        // velocity = (bpm / 240) * speed * playback_ratio = (120 / 240) * 1 * 1 = 0.5
        // time = distance / velocity = 100 / 0.5 = 200 seconds
        let advance_time = start_time + TimeSpan::from_duration(Duration::from_secs_f64(200.0));
        let speed = Decimal::one();

        player.step_to(advance_time, &speed);

        // Verify that both events were applied
        // BPM should be updated to 180
        assert_eq!(
            player.playback_state().unwrap().current_bpm(),
            &Decimal::from(180),
            "BPM change event should be applied"
        );
        // Scroll should be updated to 1.5
        assert_eq!(
            player.playback_state().unwrap().current_scroll(),
            &Decimal::from(1.5),
            "Scroll change event should be applied"
        );
    }
}
