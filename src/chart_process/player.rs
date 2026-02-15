//! Chart Player Module.
//!
//! Unified player for parsed charts, managing playback state and event processing.

use std::collections::BTreeMap;
use std::ops::{Bound, RangeBounds};
use std::time::Duration;

use gametime::{TimeSpan, TimeStamp};
use strict_num_extended::{FinF64, NonNegativeF64, PositiveF64};

use crate::chart_process::processor::AllEventsIndex;
use crate::chart_process::{ChartEvent, FlowEvent, PlayheadEvent, YCoordinate};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Maximum value for `FinF64` when overflow occurs
///
/// Used for velocity, Y coordinate, and other calculations to gracefully
/// handle overflow instead of panicking.
const MAX_FIN_F64: FinF64 = FinF64::new_const(f64::MAX);

/// Maximum value for `NonNegativeF64` when overflow occurs
const MAX_NON_NEGATIVE_F64: NonNegativeF64 = NonNegativeF64::new_const(f64::MAX);

/// Unified chart player.
///
/// This player takes a parsed chart and manages all playback state and event processing.
pub struct ChartPlayer {
    // Playback state
    started_at: TimeStamp,
    last_poll_at: TimeStamp,

    // Configuration
    pub(crate) visible_range_per_bpm: VisibleRangePerBpm,
    pub(crate) visibility_range: (Bound<FinF64>, Bound<FinF64>),

    // Performance: velocity caching
    cached_velocity: Option<FinF64>,
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
    /// let visible_range = VisibleRangePerBpm::new(base_bpm, reaction_time);
    /// let chart = BmsProcessor::parse(&bms);
    ///
    /// let mut player = ChartPlayer::start(chart, visible_range, start_time);
    /// ```
    #[must_use]
    pub fn start(
        mut chart: crate::chart_process::processor::PlayableChart,
        visible_range_per_bpm: VisibleRangePerBpm,
        start_time: TimeStamp,
    ) -> Self {
        // Extract flow_events and events from chart (take ownership)
        let flow_events = std::mem::take(&mut chart.flow_events);
        let all_events = chart.events.clone();
        let init_bpm = chart.init_bpm;
        let init_speed = chart.init_speed;

        Self {
            started_at: start_time,
            last_poll_at: start_time,
            visible_range_per_bpm,
            visibility_range: (Bound::Included(FinF64::ZERO), Bound::Included(FinF64::ONE)),
            cached_velocity: None,
            velocity_dirty: true,
            preloaded_events: Vec::new(),
            all_events,
            flow_events_by_y: flow_events,
            playback_state: PlaybackState::new(
                init_bpm,
                init_speed,
                FinF64::ONE,
                FinF64::ONE,
                YCoordinate::ZERO,
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
        let prev_y = self.playback_state.progressed_y;
        let speed = self.playback_state.current_speed;
        self.step_to(now, speed);

        let cur_y = self.playback_state.progressed_y;

        // Calculate preload range: current y + visible y range
        let visible_y_length = self.visible_window_y(&speed);
        let preload_end_y = cur_y + visible_y_length;

        use std::ops::Bound::{Excluded, Included};

        // Collect events triggered at current moment
        let mut triggered_events = self.events_in_y_range((Excluded(&prev_y), Included(&cur_y)));

        self.update_preloaded_events(&FinF64::new(preload_end_y.as_f64()).unwrap_or(MAX_FIN_F64));

        // Apply Speed changes
        for event in &triggered_events {
            if let ChartEvent::SpeedChange { factor } = event.event() {
                self.playback_state.current_speed = *factor;
            }
        }

        // Sort to maintain stable order
        triggered_events.sort_by(|a, b| {
            a.position()
                .partial_cmp(b.position())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        triggered_events
    }

    /// Set visible range per BPM.
    ///
    /// Updates the visible range configuration based on BPM.
    ///
    /// # Arguments
    ///
    /// * `visible_range_per_bpm` - New visible range per BPM configuration
    pub const fn set_visible_range_per_bpm(&mut self, visible_range_per_bpm: VisibleRangePerBpm) {
        self.visible_range_per_bpm = visible_range_per_bpm;
    }

    /// Sets the visibility range for events.
    ///
    /// # Arguments
    ///
    /// * `range` - Any type implementing `RangeBounds<FinF64>`, such as:
    ///   - `0.0..1.0` - Half-open range [0.0, 1.0)
    ///   - `0.0..=1.0` - Closed range [0.0, 1.0]
    ///   - `..` - Unbounded range
    ///   - `0.0..` - Lower bound only
    ///   - `..1.0` - Upper bound only
    ///
    /// # Examples
    ///
    /// ```ignore
    /// player.set_visibility_range(0.0..=1.0);  // Default behavior
    /// player.set_visibility_range(-0.5..1.0);  // Show events past judgment line
    /// player.set_visibility_range(..);         // Show all events
    /// ```
    pub fn set_visibility_range(&mut self, range: impl RangeBounds<FinF64>) {
        self.visibility_range = (range.start_bound().cloned(), range.end_bound().cloned());
    }

    /// Gets the current visibility range.
    #[must_use]
    pub const fn visibility_range(&self) -> &(Bound<FinF64>, Bound<FinF64>) {
        &self.visibility_range
    }

    /// Set playback ratio.
    ///
    /// Controls how fast the playback advances relative to real time.
    /// Default is 1.0. Marks velocity cache as dirty.
    ///
    /// # Arguments
    ///
    /// * `ratio` - Playback ratio (>= 0)
    pub const fn set_playback_ratio(&mut self, ratio: FinF64) {
        self.mark_velocity_dirty();
        self.playback_state.playback_ratio = ratio;
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

        let view_start = *current_y;
        let view_end = *current_y + visible_window_y;

        let visible_events = self
            .all_events
            .events_in_y_range((Bound::Excluded(view_start), Bound::Included(view_end)));

        visible_events
            .iter()
            .filter_map(|event_with_pos| {
                let event_y = event_with_pos.position();
                let start_display_ratio = Self::compute_display_ratio(
                    event_y,
                    current_y,
                    &visible_window_y,
                    scroll_factor,
                );

                let end_display_ratio = if let ChartEvent::Note {
                    length: Some(length),
                    ..
                } = event_with_pos.event()
                {
                    // Calculate end_y with overflow protection
                    let end_y_value = FinF64::new(event_y.as_f64() + length.as_f64())
                        .map(strict_num_extended::FinF64::as_f64)
                        .unwrap_or(f64::MAX);
                    let end_y = YCoordinate::new(
                        NonNegativeF64::new(end_y_value).unwrap_or(MAX_NON_NEGATIVE_F64),
                    );
                    Self::compute_display_ratio(&end_y, current_y, &visible_window_y, scroll_factor)
                } else {
                    start_display_ratio.clone()
                };

                let ratio_start = start_display_ratio.value();
                let ratio_end = end_display_ratio.value();

                let is_visible = self.overlaps_visibility_range(*ratio_start, *ratio_end);

                is_visible.then_some((
                    event_with_pos.clone(),
                    start_display_ratio..=end_display_ratio,
                ))
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
        let playback_ratio = self.playback_state.playback_ratio;
        // Calculate center time with overflow protection
        let center_nanos = FinF64::new(elapsed_nanos as f64 * playback_ratio.as_f64())
            .map(|v| strict_num_extended::FinF64::as_f64(v) as u64)
            .unwrap_or(u64::MAX);
        let center = TimeSpan::from_duration(Duration::from_nanos(center_nanos));
        self.all_events
            .events_in_time_range_offset_from(center, range)
    }

    // ===== Internal Core Methods =====

    /// Calculate velocity with caching.
    ///
    /// See [`crate::chart_process`] for the formula.
    pub fn calculate_velocity(&mut self, speed: &PositiveF64) -> FinF64 {
        if self.velocity_dirty || self.cached_velocity.is_none() {
            let computed = self.compute_velocity(*speed);
            self.cached_velocity = Some(computed);
            self.velocity_dirty = false;
            computed
        } else if let Some(cached) = self.cached_velocity {
            cached
        } else {
            // This should not happen as we checked is_none above
            self.compute_velocity(*speed)
        }
    }

    /// Compute velocity without caching (internal use).
    fn compute_velocity(&self, speed: PositiveF64) -> FinF64 {
        let current_bpm = self.playback_state.current_bpm;
        let playback_ratio = self.playback_state.playback_ratio;

        if current_bpm.as_f64() <= 0.0 {
            FinF64::new(f64::EPSILON).unwrap_or(FinF64::ZERO)
        } else {
            let base = FinF64::new(current_bpm.as_f64() / 240.0).unwrap_or(FinF64::ZERO);
            let v1 = (base * speed).unwrap_or(MAX_FIN_F64);
            let v = (v1 * playback_ratio).unwrap_or(MAX_FIN_F64);
            FinF64::new(v.as_f64().max(f64::EPSILON)).unwrap_or(MAX_FIN_F64)
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
            .and_then(|(y, events)| events.first().cloned().map(|evt| (*y, evt)))
    }

    /// Get the next flow event Y position after the given Y (exclusive).
    #[must_use]
    fn next_flow_event_y_after(&self, y_from_exclusive: YCoordinate) -> Option<YCoordinate> {
        use std::ops::Bound::{Excluded, Unbounded};
        self.flow_events_by_y
            .range((Excluded(y_from_exclusive), Unbounded))
            .next()
            .map(|(y, _)| *y)
    }

    /// Apply all flow events at the given Y position.
    fn apply_flow_events_at(&mut self, y: YCoordinate) {
        // Remove events from the map to take ownership, avoiding borrow conflicts
        if let Some(events) = self.flow_events_by_y.remove(&y) {
            for event in events {
                self.apply_flow_event(event);
            }
            // Note: events are not re-inserted since they've been applied
        }
    }

    /// Apply a flow event to this player.
    const fn apply_flow_event(&mut self, event: FlowEvent) {
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
    fn step_to(&mut self, now: TimeStamp, speed: PositiveF64) {
        let last = self.last_poll_at;
        if now <= last {
            return;
        }

        let mut remaining_time = now - last;
        let mut cur_vel = self.calculate_velocity(&speed);
        let mut cur_y = self.playback_state.progressed_y;

        // Advance in segments until time slice is used up
        loop {
            let cur_y_now = cur_y;
            let next_event_y = self.next_flow_event_y_after(cur_y_now);

            if next_event_y.is_none() || cur_vel.as_f64() <= 0.0 || remaining_time <= TimeSpan::ZERO
            {
                // Advance directly to the end with overflow protection
                let nanos = remaining_time.as_nanos().max(0) as f64;
                let time_secs = nanos / NANOS_PER_SECOND as f64;
                // Use checked multiplication via FinF64 to detect overflow
                let delta_y = if time_secs.is_finite() {
                    FinF64::new(cur_vel.as_f64() * time_secs)
                        .map(strict_num_extended::FinF64::as_f64)
                        .unwrap_or(f64::MAX)
                } else {
                    f64::MAX
                };
                cur_y = YCoordinate::new(
                    NonNegativeF64::new(cur_y_now.as_f64() + delta_y)
                        .unwrap_or(MAX_NON_NEGATIVE_F64),
                );
                break;
            }

            let Some(event_y) = next_event_y else {
                // Handle remaining time with overflow protection
                let nanos = remaining_time.as_nanos().max(0) as f64;
                let time_secs = nanos / NANOS_PER_SECOND as f64;
                let delta_y = if time_secs.is_finite() {
                    FinF64::new(cur_vel.as_f64() * time_secs)
                        .map(strict_num_extended::FinF64::as_f64)
                        .unwrap_or(f64::MAX)
                } else {
                    f64::MAX
                };
                cur_y = YCoordinate::new(
                    NonNegativeF64::new(cur_y_now.as_f64() + delta_y)
                        .unwrap_or(MAX_NON_NEGATIVE_F64),
                );
                break;
            };

            if event_y <= cur_y_now {
                // Defense: avoid infinite loop if event position doesn't advance
                // Apply all events at this Y position
                self.apply_flow_events_at(event_y);
                cur_vel = self.calculate_velocity(&speed);
                cur_y = cur_y_now;
                continue;
            }

            // Time required to reach event
            let distance = event_y - cur_y_now;
            if cur_vel.as_f64() > 0.0 {
                // Calculate time with overflow protection
                let time_secs = distance.as_f64() / cur_vel.as_f64();
                let time_to_event_nanos = if time_secs.is_finite() {
                    FinF64::new(time_secs * NANOS_PER_SECOND as f64)
                        .map(|v| v.as_f64() as u64)
                        .unwrap_or(u64::MAX)
                } else {
                    u64::MAX
                };
                let time_to_event =
                    TimeSpan::from_duration(Duration::from_nanos(time_to_event_nanos));

                if time_to_event <= remaining_time {
                    // First advance to event point
                    cur_y = event_y;
                    remaining_time -= time_to_event;
                    // Apply all events at this Y position
                    self.apply_flow_events_at(event_y);
                    cur_vel = self.calculate_velocity(&speed);
                    continue;
                }
            }

            // Time not enough to reach event, advance and end with overflow protection
            let nanos = remaining_time.as_nanos().max(0) as f64;
            let time_secs = nanos / NANOS_PER_SECOND as f64;
            let delta_y = if time_secs.is_finite() {
                FinF64::new(cur_vel.as_f64() * time_secs)
                    .map(strict_num_extended::FinF64::as_f64)
                    .unwrap_or(f64::MAX)
            } else {
                f64::MAX
            };
            cur_y = YCoordinate::new(
                NonNegativeF64::new(cur_y_now.as_f64() + delta_y).unwrap_or(MAX_NON_NEGATIVE_F64),
            );
            break;
        }

        // Update playback state
        self.playback_state.progressed_y = cur_y;
        self.last_poll_at = now;
    }

    /// Get visible window length in Y units.
    #[must_use]
    pub fn visible_window_y(&self, speed: &PositiveF64) -> YCoordinate {
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
    pub fn update_preloaded_events(&mut self, preload_end_y: &FinF64) {
        use std::ops::Bound::{Excluded, Included};

        let cur_y = self.playback_state.progressed_y;
        let preload_end_y_coord = YCoordinate::new(
            NonNegativeF64::new(preload_end_y.as_f64()).unwrap_or(MAX_NON_NEGATIVE_F64),
        );
        let new_preloaded_events = self
            .all_events
            .events_in_y_range((Excluded(&cur_y), Included(&preload_end_y_coord)));

        self.preloaded_events = new_preloaded_events;
    }

    /// Get preloaded events.
    #[must_use]
    pub const fn preloaded_events(&self) -> &Vec<PlayheadEvent> {
        &self.preloaded_events
    }

    /// Checks if a note's position overlaps with the visibility range.
    fn overlaps_visibility_range(&self, ratio_start: FinF64, ratio_end: FinF64) -> bool {
        let note_min = if ratio_start < ratio_end {
            ratio_start
        } else {
            ratio_end
        };
        let note_max = if ratio_start > ratio_end {
            ratio_start
        } else {
            ratio_end
        };

        let (vis_min, vis_max) = &self.visibility_range;
        let is_already_end = match vis_min {
            Bound::Unbounded => false,
            Bound::Included(min) => note_max < *min,
            Bound::Excluded(min) => note_max <= *min,
        };
        let is_not_started_yet = match vis_max {
            Bound::Unbounded => false,
            Bound::Included(max) => *max < note_min,
            Bound::Excluded(max) => *max <= note_min,
        };
        !(is_already_end || is_not_started_yet)
    }

    /// Compute display ratio for an event.
    #[must_use]
    pub fn compute_display_ratio(
        event_y: &YCoordinate,
        current_y: &YCoordinate,
        visible_window_y: &YCoordinate,
        scroll_factor: &FinF64,
    ) -> DisplayRatio {
        let window_value = *visible_window_y.value();
        if window_value.as_f64() > 0.0 {
            let ratio_value = FinF64::new(
                (event_y.as_f64() - current_y.as_f64()) / window_value.as_f64()
                    * scroll_factor.as_f64(),
            )
            .unwrap_or(FinF64::ZERO);
            DisplayRatio::from(ratio_value)
        } else {
            // Should not happen theoretically; indicates configuration issue if it does
            DisplayRatio::at_judgment_line()
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
    pub current_bpm: PositiveF64,
    /// Current speed factor (BMS only, BMSON always 1.0)
    pub current_speed: PositiveF64,
    /// Current scroll factor
    pub current_scroll: FinF64,
    /// Current playback ratio
    pub playback_ratio: FinF64,
    /// Current Y position in chart
    pub progressed_y: YCoordinate,
}

impl PlaybackState {
    /// Create a new playback state.
    #[must_use]
    pub const fn new(
        current_bpm: PositiveF64,
        current_speed: PositiveF64,
        current_scroll: FinF64,
        playback_ratio: FinF64,
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
    pub const fn current_bpm(&self) -> &PositiveF64 {
        &self.current_bpm
    }

    /// Get current speed factor.
    #[must_use]
    pub const fn current_speed(&self) -> &PositiveF64 {
        &self.current_speed
    }

    /// Get current scroll factor.
    #[must_use]
    pub const fn current_scroll(&self) -> &FinF64 {
        &self.current_scroll
    }

    /// Get playback ratio.
    #[must_use]
    pub const fn playback_ratio(&self) -> &FinF64 {
        &self.playback_ratio
    }

    /// Get current Y position.
    #[must_use]
    pub const fn progressed_y(&self) -> &YCoordinate {
        &self.progressed_y
    }
}

/// Visible range per BPM, representing the relationship between BPM and visible Y range.
/// See [`crate::chart_process`] for the formula.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleRangePerBpm {
    value: FinF64,
    base_bpm: FinF64,
    reaction_time_seconds: FinF64,
}

impl AsRef<FinF64> for VisibleRangePerBpm {
    fn as_ref(&self) -> &FinF64 {
        &self.value
    }
}

impl VisibleRangePerBpm {
    /// Create a new `VisibleRangePerBpm` from base BPM and reaction time.
    /// See [`crate::chart_process`] for the formula.
    #[must_use]
    pub fn new(base_bpm: &PositiveF64, reaction_time: TimeSpan) -> Self {
        // Check for effectively zero BPM to avoid division by extremely small numbers
        if base_bpm.as_f64() <= f64::EPSILON {
            Self {
                value: FinF64::ZERO,
                base_bpm: FinF64::ZERO,
                reaction_time_seconds: FinF64::ZERO,
            }
        } else {
            let reaction_time_seconds =
                FinF64::new(reaction_time.as_nanos().max(0) as f64 / NANOS_PER_SECOND as f64)
                    .unwrap_or(FinF64::ZERO);
            // Calculate value step by step with overflow protection
            // Formula: reaction_time_seconds * 240.0 / base_bpm
            let step1 = FinF64::new(reaction_time_seconds.as_f64() * 240.0).unwrap_or(MAX_FIN_F64);
            let value = FinF64::new(step1.as_f64() / base_bpm.as_f64()).unwrap_or(MAX_FIN_F64);
            Self {
                value,
                base_bpm: FinF64::new(base_bpm.as_f64()).unwrap_or(FinF64::ONE),
                reaction_time_seconds,
            }
        }
    }

    /// Returns a reference to the contained value.
    #[must_use]
    pub const fn value(&self) -> &FinF64 {
        &self.value
    }

    /// Consumes self and returns the contained value.
    #[must_use]
    pub const fn into_value(self) -> FinF64 {
        self.value
    }

    /// Calculate visible window length in y units based on current BPM, speed, and playback ratio.
    /// See [`crate::chart_process`] for formula.
    /// This ensures events stay in visible window for exactly `reaction_time` duration.
    #[must_use]
    pub fn window_y(
        &self,
        current_bpm: &PositiveF64,
        current_speed: &PositiveF64,
        playback_ratio: &FinF64,
    ) -> YCoordinate {
        // Check for invalid BPM early
        if current_bpm.as_f64() <= f64::EPSILON {
            return YCoordinate::ZERO;
        }

        // Calculate speed factor with overflow protection
        let speed_factor =
            FinF64::new(current_speed.as_f64() * playback_ratio.as_f64()).unwrap_or(MAX_FIN_F64);

        // Goal: time = reaction_time * base_bpm / current_bpm
        // velocity = (current_bpm / 240) * speed_factor
        // visible_window_y = velocity * time
        //                  = (current_bpm / 240) * speed_factor * reaction_time * base_bpm / current_bpm
        //                  = (speed_factor / 240) * reaction_time * base_bpm

        // Calculate velocity step by step with overflow checks
        let bpm_div_240 = FinF64::new(current_bpm.as_f64() / 240.0).unwrap_or(MAX_FIN_F64);
        let velocity =
            FinF64::new(bpm_div_240.as_f64() * speed_factor.as_f64()).unwrap_or(MAX_FIN_F64);

        // Calculate adjusted value step by step to catch overflow early
        // Formula: velocity * reaction_time_seconds * base_bpm / current_bpm
        let step1 = FinF64::new(velocity.as_f64() * self.reaction_time_seconds.as_f64())
            .unwrap_or(MAX_FIN_F64);
        let step2 = FinF64::new(step1.as_f64() * self.base_bpm.as_f64()).unwrap_or(MAX_FIN_F64);
        let adjusted = FinF64::new(step2.as_f64() / current_bpm.as_f64()).unwrap_or(MAX_FIN_F64);

        YCoordinate::new(NonNegativeF64::new(adjusted.as_f64()).unwrap_or(MAX_NON_NEGATIVE_F64))
    }

    /// Calculate reaction time from visible range per BPM.
    /// See [`crate::chart_process`] for the formula.
    #[must_use]
    pub fn to_reaction_time(&self) -> TimeSpan {
        if self.reaction_time_seconds.as_f64() == 0.0 {
            TimeSpan::ZERO
        } else {
            let nanos = (self.reaction_time_seconds.as_f64() * NANOS_PER_SECOND as f64) as u64;
            TimeSpan::from_duration(Duration::from_nanos(nanos))
        }
    }
}

impl From<FinF64> for VisibleRangePerBpm {
    fn from(value: FinF64) -> Self {
        let base_bpm = FinF64::ONE;
        let reaction_time_seconds = (value / 240.0).unwrap_or(FinF64::ZERO);
        Self {
            value,
            base_bpm,
            reaction_time_seconds,
        }
    }
}

impl From<VisibleRangePerBpm> for FinF64 {
    fn from(value: VisibleRangePerBpm) -> Self {
        value.value
    }
}

/// Display ratio wrapper type, representing the actual position of a note in the display area.
///
/// 0 is the judgment line, 1 is the position where the note generally starts to appear.
/// See [`crate::chart_process`] for the formula.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct DisplayRatio(pub FinF64);

impl AsRef<FinF64> for DisplayRatio {
    fn as_ref(&self) -> &FinF64 {
        &self.0
    }
}

impl DisplayRatio {
    /// Create a new `DisplayRatio`
    #[must_use]
    pub const fn new(value: FinF64) -> Self {
        Self(value)
    }

    /// Returns a reference to the contained value.
    #[must_use]
    pub const fn value(&self) -> &FinF64 {
        &self.0
    }

    /// Consumes self and returns the contained value.
    #[must_use]
    pub const fn into_value(self) -> FinF64 {
        self.0
    }

    /// Create a `DisplayRatio` representing the judgment line (value 0)
    #[must_use]
    pub const fn at_judgment_line() -> Self {
        Self(FinF64::ZERO)
    }

    /// Create a `DisplayRatio` representing the position where note starts to appear (value 1)
    #[must_use]
    pub const fn at_appearance() -> Self {
        Self(FinF64::ONE)
    }
}

impl From<FinF64> for DisplayRatio {
    fn from(value: FinF64) -> Self {
        Self(value)
    }
}

impl From<DisplayRatio> for FinF64 {
    fn from(value: DisplayRatio) -> Self {
        value.0
    }
}

impl From<f64> for DisplayRatio {
    fn from(value: f64) -> Self {
        Self(FinF64::new(value).unwrap_or(FinF64::ZERO))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use super::*;
    use crate::chart_process::YCoordinate;
    use crate::chart_process::processor::{ChartResources, PlayableChart};
    use strict_num_extended::{FinF64, NonNegativeF64, PositiveF64};

    /// Default test BPM value (120.0) - used as initial BPM
    const TEST_BPM_120: PositiveF64 = PositiveF64::new_const(120.0);
    /// Default test BPM value (180.0) - used for BPM change events
    const TEST_BPM: PositiveF64 = PositiveF64::new_const(180.0);
    /// Test scroll factor (1.5)
    const TEST_SCROLL_FACTOR: FinF64 = FinF64::new_const(1.5);
    /// Test Y event value (100.0)
    const TEST_Y_EVENT: YCoordinate = YCoordinate::new(NonNegativeF64::new_const(100.0));
    /// Test current Y value (10.0)
    const TEST_CURRENT_Y: YCoordinate = YCoordinate::new(NonNegativeF64::new_const(10.0));
    /// Test event Y value (11.0)
    const TEST_EVENT_Y: YCoordinate = YCoordinate::new(NonNegativeF64::new_const(11.0));
    /// Test visible window Y (2.0)
    const TEST_VISIBLE_WINDOW_Y: YCoordinate = YCoordinate::new(NonNegativeF64::TWO);

    #[test]
    fn test_velocity_caching() {
        let chart = PlayableChart::from_parts(
            ChartResources::new(HashMap::new(), HashMap::new()),
            AllEventsIndex::new(BTreeMap::new()),
            BTreeMap::new(),
            TEST_BPM_120,
            PositiveF64::ONE,
        );

        let mut player = ChartPlayer::start(
            chart,
            VisibleRangePerBpm::new(&TEST_BPM_120, TimeSpan::SECOND),
            TimeStamp::now(),
        );

        let speed = PositiveF64::ONE;

        // First call computes velocity
        let v1 = player.calculate_velocity(&speed);
        assert!(v1.as_f64() > 0.0);

        // Second call should use cache
        let v2 = player.calculate_velocity(&speed);
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_flow_event_application_after_start() {
        use std::collections::BTreeMap;

        let y_event = TEST_Y_EVENT;

        let mut flow_events_by_y = BTreeMap::new();
        flow_events_by_y.insert(
            y_event,
            vec![
                FlowEvent::Bpm(TEST_BPM),
                FlowEvent::Scroll(TEST_SCROLL_FACTOR),
            ],
        );

        let chart = PlayableChart::from_parts(
            ChartResources::new(HashMap::new(), HashMap::new()),
            AllEventsIndex::new(BTreeMap::new()),
            flow_events_by_y,
            TEST_BPM_120,
            PositiveF64::ONE,
        );

        let mut player = ChartPlayer::start(
            chart,
            VisibleRangePerBpm::new(&TEST_BPM_120, TimeSpan::SECOND),
            TimeStamp::now(),
        );

        // Initial state after start
        assert!((player.playback_state().current_bpm().as_f64() - 120.0).abs() < f64::EPSILON);
        assert!((player.playback_state().current_scroll().as_f64() - 1.0).abs() < f64::EPSILON);

        // Apply BPM change
        player.apply_flow_event(FlowEvent::Bpm(TEST_BPM));

        assert!((player.playback_state().current_bpm().as_f64() - 180.0).abs() < f64::EPSILON);
        assert!(player.velocity_dirty);
    }

    #[test]
    fn test_display_ratio_computation() {
        let current_y = TEST_CURRENT_Y;
        let event_y = TEST_EVENT_Y;
        let visible_window_y = TEST_VISIBLE_WINDOW_Y;
        let scroll_factor = FinF64::ONE;

        let ratio = ChartPlayer::compute_display_ratio(
            &event_y,
            &current_y,
            &visible_window_y,
            &scroll_factor,
        );

        // (11 - 10) / 2 = 0.5
        assert!((ratio.value().as_f64() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_multiple_flow_events_same_y_all_triggered() {
        use std::collections::BTreeMap;

        // Setup: Create flow events at the same Y position
        let y_event = TEST_Y_EVENT;

        let mut flow_events_by_y = BTreeMap::new();
        flow_events_by_y.insert(
            y_event,
            vec![
                FlowEvent::Bpm(TEST_BPM),
                FlowEvent::Scroll(TEST_SCROLL_FACTOR),
            ],
        );

        let chart = PlayableChart::from_parts(
            ChartResources::new(HashMap::new(), HashMap::new()),
            AllEventsIndex::new(BTreeMap::new()),
            flow_events_by_y,
            TEST_BPM_120,
            PositiveF64::ONE,
        );

        let start_time = TimeStamp::now();
        let mut player = ChartPlayer::start(
            chart,
            VisibleRangePerBpm::new(&TEST_BPM_120, TimeSpan::SECOND),
            TimeStamp::now(),
        );

        // Initial state
        assert!((player.playback_state().current_bpm().as_f64() - 120.0).abs() < f64::EPSILON);
        assert!((player.playback_state().current_scroll().as_f64() - 1.0).abs() < f64::EPSILON);

        // Advance past the event Y position
        // Calculate time needed: distance / velocity
        // velocity = (bpm / 240) * speed * playback_ratio = (120 / 240) * 1 * 1 = 0.5
        // time = distance / velocity = 100 / 0.5 = 200 seconds
        // Add a small epsilon to account for floating-point precision issues
        let advance_time =
            start_time + TimeSpan::from_duration(Duration::from_secs_f64(200.0 + 0.001));
        let speed = PositiveF64::ONE;

        player.step_to(advance_time, speed);

        // Verify that both events were applied
        // BPM should be updated to 180
        assert!(
            (player.playback_state().current_bpm().as_f64() - 180.0).abs() < f64::EPSILON,
            "BPM change event should be applied"
        );
        // Scroll should be updated to 1.5
        assert!(
            (player.playback_state().current_scroll().as_f64() - 1.5).abs() < f64::EPSILON,
            "Scroll change event should be applied"
        );
        // Scroll should be updated to 1.5
        assert!(
            (player.playback_state().current_scroll().as_f64() - 1.5).abs() < f64::EPSILON,
            "Scroll change event should be applied"
        );
    }
}
