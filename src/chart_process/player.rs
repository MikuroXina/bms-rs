//! Chart Player Module.
//!
//! Unified player for parsed charts, managing playback state and event processing.

use std::collections::BTreeMap;
use std::time::Duration;

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive, Zero};

use crate::bms::Decimal;
use crate::chart_process::types::{
    AllEventsIndex, DisplayRatio, FlowEvent, PlayheadEvent, VisibleRangePerBpm, YCoordinate,
};
use crate::chart_process::{ChartEvent, ControlEvent};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Unified chart player.
///
/// This player takes a parsed chart and manages all playback state and event processing.
pub struct ChartPlayer {
    // Playback state
    started_at: TimeStamp,
    last_poll_at: TimeStamp,

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

    // Playback state (always initialized after construction)
    playback_state: PlaybackState,
}

impl ChartPlayer {
    /// Create a new player and start playback at the given time.
    ///
    /// This is the only way to create a `ChartPlayer` instance.
    /// It combines chart initialization and playback startup into a single operation.
    ///
    /// # Arguments
    ///
    /// * `chart` - The parsed chart to play
    /// * `visible_range_per_bpm` - Visible range configuration based on BPM
    /// * `start_time` - The timestamp when playback should start
    ///
    /// # Returns
    ///
    /// A fully initialized `ChartPlayer` ready to receive `update()` calls.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use gametime::TimeStamp;
    /// use bms_rs::chart_process::{ChartPlayer, VisibleRangePerBpm, BaseBpmGenerator};
    ///
    /// let start_time = TimeStamp::now();
    /// let base_bpm = StartBpmGenerator.generate(&bms)?;
    /// let visible_range = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    /// let chart = BmsProcessor::parse(&bms);
    ///
    /// let mut player = ChartPlayer::start(chart, visible_range, start_time);
    /// ```
    #[must_use]
    pub fn start(
        mut chart: crate::chart_process::types::ParsedChart,
        visible_range_per_bpm: VisibleRangePerBpm,
        start_time: TimeStamp,
    ) -> Self {
        // Extract flow_events and events from chart (take ownership)
        let flow_events = std::mem::take(&mut chart.flow_events);
        let all_events = chart.events.clone();
        let init_bpm = chart.init_bpm.clone();
        let init_speed = chart.init_speed.clone();

        Self {
            started_at: start_time,
            last_poll_at: start_time,
            visible_range_per_bpm,
            cached_velocity: None,
            velocity_dirty: true,
            preloaded_events: Vec::new(),
            all_events,
            flow_events_by_y: flow_events,
            playback_state: PlaybackState::new(
                init_bpm,
                init_speed,
                Decimal::one(),
                Decimal::one(),
                YCoordinate::zero(),
            ),
        }
    }

    // ===== Playback Control =====

    /// Update playback to the given time, return triggered events.
    ///
    /// Advances the internal playback state from `last_poll_at` to `now`,
    /// processing all flow events and collecting triggered chart events.
    ///
    /// # Arguments
    ///
    /// * `now` - The timestamp to advance playback to (must be >= `last_poll_at`)
    ///
    /// # Returns
    ///
    /// A vector of events triggered during this time slice. May be empty if
    /// no events were triggered.
    ///
    /// # Flow Events Processing
    ///
    /// This method automatically processes BPM changes, scroll changes, and
    /// speed changes that occur during the time slice, updating the internal
    /// `playback_state` accordingly.
    pub fn update(&mut self, now: TimeStamp) -> Vec<PlayheadEvent> {
        let prev_y = self.playback_state.progressed_y.clone();
        let speed = self.playback_state.current_speed.clone();
        self.step_to(now, &speed);

        let cur_y = self.playback_state.progressed_y.clone();

        // Calculate preload range: current y + visible y range
        let visible_y_length = self.visible_window_y(&speed);
        let preload_end_y = &cur_y + &visible_y_length;

        use std::ops::Bound::{Excluded, Included};

        // Collect events triggered at current moment
        let mut triggered_events = self.events_in_y_range((Excluded(&prev_y), Included(&cur_y)));

        self.update_preloaded_events(&preload_end_y);

        // Apply Speed changes
        for event in &triggered_events {
            if let ChartEvent::SpeedChange { factor } = event.event() {
                self.playback_state.current_speed = factor.clone();
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
    /// Always returns a valid reference because the player is always in the playing state
    /// after construction via [`ChartPlayer::start()`].
    #[must_use]
    pub const fn playback_state(&self) -> &PlaybackState {
        &self.playback_state
    }

    /// Get visible range per BPM.
    #[must_use]
    pub const fn visible_range_per_bpm(&self) -> &VisibleRangePerBpm {
        &self.visible_range_per_bpm
    }

    /// Get the start time of playback.
    #[must_use]
    pub const fn started_at(&self) -> TimeStamp {
        self.started_at
    }

    // ===== Visible Events =====

    /// Get all events in current visible area (with display positions).
    ///
    /// Returns all events that are currently visible based on the playback
    /// position and reaction time window. Each event is paired with its
    /// display ratio range indicating where it appears on screen.
    ///
    /// # Returns
    ///
    /// A vector of `(event, display_ratio_range)` tuples. May be empty if
    /// no events are currently visible.
    ///
    /// # Display Ratio
    ///
    /// The display ratio ranges from 0.0 (judgment line) to 1.0+ (visible
    /// area top), with scroll factor applied.
    pub fn visible_events(
        &mut self,
    ) -> Vec<(PlayheadEvent, std::ops::RangeInclusive<DisplayRatio>)> {
        let current_y = &self.playback_state.progressed_y;
        let visible_window_y = self.visible_window_y(&self.playback_state.current_speed);
        let scroll_factor = &self.playback_state.current_scroll;

        self.preloaded_events
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
            .collect()
    }

    /// Query events in a time window.
    pub fn events_in_time_range(
        &self,
        range: impl std::ops::RangeBounds<TimeSpan>,
    ) -> Vec<PlayheadEvent> {
        let started = self.started_at;
        let last = self.last_poll_at;
        // Calculate center time: elapsed time scaled by playback ratio
        let elapsed = last
            .checked_elapsed_since(started)
            .unwrap_or(TimeSpan::ZERO);
        let elapsed_nanos = elapsed.as_nanos().max(0) as u64;
        let elapsed_nanos = Decimal::from(elapsed_nanos);
        let playback_ratio = self.playback_state.playback_ratio.clone();
        let center_nanos = (elapsed_nanos * playback_ratio).to_u64().unwrap_or(0);
        let center = TimeSpan::from_duration(Duration::from_nanos(center_nanos));
        self.all_events
            .events_in_time_range_offset_from(center, range)
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
        let current_bpm = self.playback_state.current_bpm.clone();
        let playback_ratio = self.playback_state.playback_ratio.clone();

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
                self.playback_state.current_bpm = bpm;
            }
            FlowEvent::Speed(_s) => {
                // Speed is format-specific (BMS only)
                // Handled in update() method
            }
            FlowEvent::Scroll(s) => {
                self.playback_state.current_scroll = s;
                // Scroll doesn't affect velocity
            }
        }
    }

    /// Advance time to `now`, performing segmented integration.
    ///
    /// This is the core time progression algorithm, shared between BMS and BMSON.
    fn step_to(&mut self, now: TimeStamp, speed: &Decimal) {
        let last = self.last_poll_at;
        if now <= last {
            return;
        }

        let mut remaining_time = now - last;
        let mut cur_vel = self.calculate_velocity(speed);
        let mut cur_y = self.playback_state.progressed_y.clone();

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
        self.playback_state.progressed_y = cur_y;
        self.last_poll_at = now;
    }

    /// Get visible window length in Y units.
    #[must_use]
    pub fn visible_window_y(&self, speed: &Decimal) -> YCoordinate {
        self.visible_range_per_bpm.window_y(
            &self.playback_state.current_bpm,
            speed,
            &self.playback_state.playback_ratio,
        )
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

        let cur_y = &self.playback_state.progressed_y;
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
                self.playback_state.playback_ratio = ratio;
            }
        }
    }
}

/// Playback state snapshot.
///
/// Represents the current playback state of the player, including all
/// flow parameters and position information. This state is only available
/// after `start_play()` has been called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackState {
    /// Current BPM value
    pub current_bpm: Decimal,
    /// Current speed factor (BMS only, BMSON always 1.0)
    pub current_speed: Decimal,
    /// Current scroll factor
    pub current_scroll: Decimal,
    /// Current playback ratio
    pub playback_ratio: Decimal,
    /// Current Y position in chart
    pub progressed_y: YCoordinate,
}

impl PlaybackState {
    /// Create a new playback state.
    #[must_use]
    pub const fn new(
        current_bpm: Decimal,
        current_speed: Decimal,
        current_scroll: Decimal,
        playback_ratio: Decimal,
        progressed_y: YCoordinate,
    ) -> Self {
        Self {
            current_bpm,
            current_speed,
            current_scroll,
            playback_ratio,
            progressed_y,
        }
    }

    /// Get current BPM.
    #[must_use]
    pub const fn current_bpm(&self) -> &Decimal {
        &self.current_bpm
    }

    /// Get current speed factor.
    #[must_use]
    pub const fn current_speed(&self) -> &Decimal {
        &self.current_speed
    }

    /// Get current scroll factor.
    #[must_use]
    pub const fn current_scroll(&self) -> &Decimal {
        &self.current_scroll
    }

    /// Get playback ratio.
    #[must_use]
    pub const fn playback_ratio(&self) -> &Decimal {
        &self.playback_ratio
    }

    /// Get current Y position.
    #[must_use]
    pub const fn progressed_y(&self) -> &YCoordinate {
        &self.progressed_y
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

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

        let mut player = ChartPlayer::start(
            chart,
            VisibleRangePerBpm::new(&BaseBpm::new(Decimal::from(120)), TimeSpan::SECOND),
            TimeStamp::now(),
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

        let mut player = ChartPlayer::start(
            chart,
            VisibleRangePerBpm::new(&BaseBpm::new(Decimal::from(120)), TimeSpan::SECOND),
            TimeStamp::now(),
        );

        // Initial state after start
        assert_eq!(player.playback_state().current_bpm(), &Decimal::from(120));
        assert_eq!(player.playback_state().current_scroll(), &Decimal::one());

        // Apply BPM change
        player.apply_flow_event(FlowEvent::Bpm(Decimal::from(180)));

        assert_eq!(player.playback_state().current_bpm(), &Decimal::from(180));
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

        let start_time = TimeStamp::now();
        let mut player = ChartPlayer::start(
            chart,
            VisibleRangePerBpm::new(&BaseBpm::new(Decimal::from(120)), TimeSpan::SECOND),
            start_time,
        );

        // Initial state
        assert_eq!(player.playback_state().current_bpm(), &Decimal::from(120));
        assert_eq!(player.playback_state().current_scroll(), &Decimal::one());

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
            player.playback_state().current_bpm(),
            &Decimal::from(180),
            "BPM change event should be applied"
        );
        // Scroll should be updated to 1.5
        assert_eq!(
            player.playback_state().current_scroll(),
            &Decimal::from(1.5),
            "Scroll change event should be applied"
        );
    }
}
