//! Chart player module
//!
//! Provides traits and implementations for playing pre-parsed chart events.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::path::Path;
use std::time::Duration;

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive, Zero};

use crate::bms::Decimal;
use crate::chart_process::NANOS_PER_SECOND;

use super::base_bpm::VisibleRangePerBpm;
use super::resource::{BmpId, ResourceMapping, WavId};
use super::{
    AllEventsIndex, ControlEvent, DisplayRatio, EventParseOutput, PlayheadEvent, YCoordinate,
};

/// Universal chart player implementation.
///
/// This is a generic player that works with any resource mapping implementation.
/// It uses the `ProcessorCore` for all playback logic and delegates resource
/// queries to the `ResourceMapping` implementation.
pub struct UniversalChartPlayer<R: ResourceMapping> {
    /// Resource mapping
    resources: R,

    /// Core processor logic
    core: ProcessorCore,
}

impl<R: ResourceMapping> UniversalChartPlayer<R> {
    /// Create a new universal chart player.
    ///
    /// # Arguments
    /// * `all_events` - Precomputed event index
    /// * `flow_events_by_y` - Flow events (BPM/Speed/Scroll) indexed by Y coordinate
    /// * `init_bpm` - Initial BPM value
    /// * `visible_range_per_bpm` - Visible range configuration
    /// * `resources` - Resource mapping implementation
    #[must_use]
    pub fn new(
        all_events: super::AllEventsIndex,
        flow_events_by_y: BTreeMap<YCoordinate, Vec<FlowEvent>>,
        init_bpm: Decimal,
        visible_range_per_bpm: VisibleRangePerBpm,
        resources: R,
    ) -> Self {
        let core = ProcessorCore::new(
            init_bpm,
            visible_range_per_bpm,
            all_events,
            flow_events_by_y,
        );

        Self { resources, core }
    }

    /// Create a universal chart player from processor output.
    ///
    /// This is a convenience method that creates a player from the output
    /// of a `ChartProcessor::process()` call.
    ///
    /// # Arguments
    /// * `output` - The process output from a chart processor
    /// * `visible_range_per_bpm` - Visible range configuration
    #[must_use]
    pub fn from_parse_output(
        output: EventParseOutput<R>,
        visible_range_per_bpm: VisibleRangePerBpm,
    ) -> Self {
        Self::new(
            output.all_events,
            output.flow_events_by_y,
            output.init_bpm,
            visible_range_per_bpm,
            output.resources,
        )
    }

    /// Check if playback has started.
    ///
    /// # Returns
    ///
    /// - `true` - playback has started (i.e., `start_play()` was called)
    /// - `false` - playback has not started
    #[must_use]
    pub const fn is_playing(&self) -> bool {
        self.core.started_at().is_some()
    }

    /// Get the current playback head position (Y coordinate).
    ///
    /// # Returns
    ///
    /// Returns the current position of the playback head on the timeline,
    /// represented as a Y coordinate. The Y coordinate is the accumulated
    /// position calculated from the chart start.
    #[must_use]
    pub const fn current_y(&self) -> &YCoordinate {
        self.core.progressed_y()
    }

    /// Reset the player state.
    ///
    /// Clears playback state and returns the player to its initial state.
    ///
    /// # Note
    ///
    /// This method will:
    /// - Clear the playback start time
    /// - Clear the last poll time
    /// - Reset the playback head position to 0
    /// - Clear preloaded events
    ///
    /// This method will **not** reset:
    /// - BPM value
    /// - Speed factor
    /// - Scroll factor
    /// - Playback ratio
    /// - Visible range configuration
    pub fn reset(&mut self) {
        self.core.reset();
    }

    /// Start playback at the given time.
    pub fn start_play(&mut self, now: TimeStamp) {
        self.core.start_play(now);
    }

    /// Get the playback start time.
    ///
    /// Returns `None` if playback has not started.
    #[must_use]
    pub const fn started_at(&self) -> Option<TimeStamp> {
        self.core.started_at()
    }

    /// Get the current BPM.
    #[must_use]
    pub const fn current_bpm(&self) -> &Decimal {
        &self.core.current_bpm
    }

    /// Get the current speed factor.
    #[must_use]
    pub const fn current_speed(&self) -> &Decimal {
        &self.core.current_speed
    }

    /// Get the current scroll factor.
    #[must_use]
    pub const fn current_scroll(&self) -> &Decimal {
        &self.core.current_scroll
    }

    /// Get the playback ratio.
    #[must_use]
    pub const fn playback_ratio(&self) -> &Decimal {
        &self.core.playback_ratio
    }

    /// Get the visible range per BPM configuration.
    #[must_use]
    pub const fn visible_range_per_bpm(&self) -> &VisibleRangePerBpm {
        &self.core.visible_range_per_bpm
    }

    /// Iterate over all audio file mappings.
    ///
    /// This is more efficient than collecting into a `HashMap`, as it avoids
    /// intermediate allocations. Use this for processing all audio files.
    pub fn for_each_audio_file<F>(&self, f: F)
    where
        F: FnMut(WavId, &Path),
    {
        self.resources.for_each_wav_path(f);
    }

    /// Iterate over all BMP file mappings.
    ///
    /// This is more efficient than collecting into a `HashMap`, as it avoids
    /// intermediate allocations. Use this for processing all image files.
    pub fn for_each_bmp_file<F>(&self, f: F)
    where
        F: FnMut(BmpId, &Path),
    {
        self.resources.for_each_bmp_path(f);
    }

    /// Update playback to the given time and return triggered events.
    ///
    /// Returns an iterator of events that were triggered between the last update
    /// and the current time.
    pub fn update(&mut self, now: TimeStamp) -> impl Iterator<Item = PlayheadEvent> {
        self.core.update_base(now).into_iter()
    }

    /// Get events in a time range.
    ///
    /// The range is relative to the chart start time (`activate_time`).
    pub fn events_in_time_range(
        &self,
        range: impl std::ops::RangeBounds<TimeSpan>,
    ) -> impl Iterator<Item = PlayheadEvent> {
        self.core.events_in_time_range(range).into_iter()
    }

    /// Post control events to the player.
    ///
    /// Control events can modify playback parameters like visible range or playback ratio.
    pub fn post_events(&mut self, events: impl Iterator<Item = ControlEvent>) {
        events.for_each(|event| self.core.handle_control_event(event));
    }

    /// Get currently visible events with their display ratios.
    ///
    /// Returns an iterator of tuples containing:
    /// - A reference to the event with its position
    /// - The display ratio range (start..=end) for rendering
    ///
    /// For normal notes and events, start and end are the same.
    /// For long notes, the range represents the note's length.
    pub fn visible_events(
        &self,
    ) -> impl Iterator<Item = (&PlayheadEvent, RangeInclusive<DisplayRatio>)> {
        self.core.compute_visible_events().into_iter()
    }
}

/// Flow events that affect playback speed/scroll.
#[derive(Debug, Clone)]
pub enum FlowEvent {
    /// BPM change event.
    Bpm(Decimal),
    /// Speed factor change event (BMS only).
    Speed(Decimal),
    /// Scroll factor change event.
    Scroll(Decimal),
}

impl FlowEvent {
    /// Apply this flow event to the processor core.
    pub fn apply_to(&self, core: &mut ProcessorCore) {
        match self {
            FlowEvent::Bpm(bpm) => {
                core.current_bpm = bpm.clone();
                core.mark_velocity_dirty();
            }
            FlowEvent::Speed(s) => {
                core.current_speed = s.clone();
                core.mark_velocity_dirty();
            }
            FlowEvent::Scroll(s) => {
                core.current_scroll = s.clone();
                // Scroll doesn't affect velocity
            }
        }
    }

    /// Get the priority of this flow event.
    ///
    /// Lower values indicate higher priority. The priority order is:
    /// - BPM (0): Most fundamental parameter, must be applied first
    /// - Speed (1): Depends on BPM, as velocity = (BPM/240) * Speed
    /// - Scroll (2): Doesn't affect velocity, only display
    const fn priority(&self) -> u8 {
        match self {
            FlowEvent::Bpm(_) => 0,
            FlowEvent::Speed(_) => 1,
            FlowEvent::Scroll(_) => 2,
        }
    }

    /// Sort multiple flow events by priority.
    ///
    /// Events will be sorted in-place according to their priority:
    /// BPM first, then Speed, then Scroll.
    ///
    /// # Arguments
    /// * `events` - The events to sort (modified in-place)
    pub fn sort_by_priority(events: &mut [FlowEvent]) {
        events.sort_by_key(FlowEvent::priority);
    }
}

/// Shared core processor logic.
///
/// This struct contains all the common state and logic shared between
/// `BmsProcessor` and `BmsonProcessor`, including:
/// - Playback state management
/// - Time progression (`step_to`)
/// - Velocity calculation with caching
/// - Visible events computation
pub struct ProcessorCore {
    // Playback state
    pub(crate) started_at: Option<TimeStamp>,
    pub(crate) last_poll_at: Option<TimeStamp>,
    pub(crate) progressed_y: YCoordinate,

    // Flow parameters
    pub(crate) current_bpm: Decimal,
    pub(crate) current_scroll: Decimal,
    pub(crate) current_speed: Decimal,
    pub(crate) playback_ratio: Decimal,
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
}

impl ProcessorCore {
    /// Create a new processor core.
    #[must_use]
    pub(crate) fn new(
        init_bpm: Decimal,
        visible_range_per_bpm: VisibleRangePerBpm,
        all_events: AllEventsIndex,
        flow_events_by_y: BTreeMap<YCoordinate, Vec<FlowEvent>>,
    ) -> Self {
        Self {
            started_at: None,
            last_poll_at: None,
            progressed_y: YCoordinate::zero(),
            current_bpm: init_bpm.clone(),
            current_scroll: Decimal::one(),
            current_speed: Decimal::one(),
            playback_ratio: Decimal::one(),
            visible_range_per_bpm,
            cached_velocity: None,
            velocity_dirty: true,
            preloaded_events: Vec::new(),
            all_events,
            flow_events_by_y,
            init_bpm,
        }
    }

    /// Start playback at the given time.
    pub fn start_play(&mut self, now: TimeStamp) {
        self.started_at = Some(now);
        self.last_poll_at = Some(now);
        self.progressed_y = YCoordinate::zero();
        self.preloaded_events.clear();
        self.current_bpm = self.init_bpm.clone();
        self.current_scroll = Decimal::one();
        self.current_speed = Decimal::one();
        self.mark_velocity_dirty();
    }

    /// Reset the processor core state.
    ///
    /// Clears playback state but preserves flow parameters (BPM, Speed, Scroll, etc.).
    pub fn reset(&mut self) {
        self.started_at = None;
        self.last_poll_at = None;
        self.progressed_y = YCoordinate::zero();
        self.preloaded_events.clear();
    }

    /// Get the playback start time.
    #[must_use]
    pub const fn started_at(&self) -> Option<TimeStamp> {
        self.started_at
    }

    /// Get the last poll time.
    #[must_use]
    pub const fn last_poll_at(&self) -> Option<TimeStamp> {
        self.last_poll_at
    }

    /// Set the last poll time.
    pub const fn set_last_poll_at(&mut self, time: TimeStamp) {
        self.last_poll_at = Some(time);
    }

    /// Core update logic shared by BMS and BMSON processors.
    ///
    /// This method advances the timeline and returns triggered events.
    pub fn update_base(&mut self, now: TimeStamp) -> Vec<PlayheadEvent> {
        let prev_y = self.progressed_y().clone();
        self.step_to(now);
        let cur_y = self.progressed_y();

        // Calculate preload range: current y + visible y range
        let visible_y_length = self.visible_window_y();
        let preload_end_y = cur_y + &visible_y_length;

        use std::ops::Bound::{Excluded, Included};

        // Collect events triggered at current moment
        let triggered_events = self.events_in_y_range((Excluded(&prev_y), Included(cur_y)));

        self.update_preloaded_events(&preload_end_y);

        triggered_events
    }

    /// Get the current Y position.
    #[must_use]
    pub const fn progressed_y(&self) -> &YCoordinate {
        &self.progressed_y
    }

    /// Calculate velocity with caching.
    ///
    /// Formula: `velocity = (bpm / 240) * speed * playback_ratio`
    pub fn calculate_velocity(&mut self) -> Decimal {
        if self.velocity_dirty || self.cached_velocity.is_none() {
            let computed = self.compute_velocity();
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
    fn compute_velocity(&self) -> Decimal {
        if self.current_bpm <= Decimal::zero() {
            Decimal::from(f64::EPSILON)
        } else {
            let denom = Decimal::from(240);
            let base = &self.current_bpm / &denom;
            let v1 = base * self.current_speed.clone();
            let v = &v1 * &self.playback_ratio;
            v.max(Decimal::from(f64::EPSILON))
        }
    }

    /// Mark velocity cache as dirty.
    pub const fn mark_velocity_dirty(&mut self) {
        self.velocity_dirty = true;
    }

    /// Get the next flow events after the given Y position (exclusive).
    ///
    /// Returns all flow events at the next Y position, as there may be multiple
    /// events (BPM, Speed, Scroll) at the same position.
    #[must_use]
    pub fn next_flow_events_after(
        &self,
        y_from_exclusive: &YCoordinate,
    ) -> Option<(YCoordinate, Vec<FlowEvent>)> {
        use std::ops::Bound::{Excluded, Unbounded};
        self.flow_events_by_y
            .range((Excluded(y_from_exclusive), Unbounded))
            .next()
            .map(|(y, events)| (y.clone(), events.clone()))
    }

    /// Advance time to `now`, performing segmented integration.
    ///
    /// This is the core time progression algorithm, shared between BMS and BMSON.
    pub fn step_to(&mut self, now: TimeStamp) {
        let Some(started) = self.started_at else {
            return;
        };
        let last = self.last_poll_at.unwrap_or(started);
        if now <= last {
            return;
        }

        let mut remaining_time = now - last;
        let mut cur_vel = self.calculate_velocity();
        let mut cur_y = self.progressed_y.clone();

        // Prevent infinite loops in edge cases
        let mut iterations = 0usize;
        const MAX_ITERATIONS: usize = 1000;

        // Advance in segments until time slice is used up
        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                // Force exit to prevent infinite loop
                break;
            }
            let cur_y_now = cur_y.clone();
            let next_events = self.next_flow_events_after(&cur_y_now);

            if next_events.is_none()
                || cur_vel <= Decimal::zero()
                || remaining_time <= TimeSpan::ZERO
            {
                // Advance directly to the end
                let delta_y = (cur_vel * Decimal::from(remaining_time.as_nanos().max(0)))
                    / Decimal::from(NANOS_PER_SECOND);
                cur_y = cur_y_now + YCoordinate::new(delta_y.round());
                break;
            }

            let Some((event_y, events)) = next_events else {
                let delta_y = (cur_vel * Decimal::from(remaining_time.as_nanos().max(0)))
                    / Decimal::from(NANOS_PER_SECOND);
                cur_y = cur_y_now + YCoordinate::new(delta_y.round());
                break;
            };

            if event_y <= cur_y_now {
                // Defense: avoid infinite loop if event position doesn't advance
                // Apply all events at this position
                let mut sorted_events = events;
                FlowEvent::sort_by_priority(&mut sorted_events);
                for evt in sorted_events {
                    evt.apply_to(self);
                }
                cur_vel = self.calculate_velocity();
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
                    cur_y = event_y;
                    remaining_time -= time_to_event;
                    // Apply all events at this position
                    let mut sorted_events = events;
                    FlowEvent::sort_by_priority(&mut sorted_events);
                    for evt in sorted_events {
                        evt.apply_to(self);
                    }
                    cur_vel = self.calculate_velocity();
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

        self.progressed_y = cur_y;
        self.last_poll_at = Some(now);
    }

    /// Get visible window length in Y units.
    #[must_use]
    pub fn visible_window_y(&self) -> YCoordinate {
        self.visible_range_per_bpm.window_y(
            &self.current_bpm,
            &self.current_speed,
            &self.playback_ratio,
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
    ///
    /// This method incrementally updates the preloaded events:
    /// 1. Removes events that have already passed (position <= current Y)
    /// 2. Adds new events that have entered the preload range
    ///
    /// This is more efficient than completely rebuilding the vector on every call.
    pub fn update_preloaded_events(&mut self, preload_end_y: &YCoordinate) {
        use std::ops::Bound::{Excluded, Included};

        let cur_y = &self.progressed_y;

        // Remove events that have already passed
        let cutoff_idx = self
            .preloaded_events
            .partition_point(|ev| ev.position() <= cur_y);
        self.preloaded_events.drain(..cutoff_idx);

        // Get new events that have entered the preload range
        // Only query from the last preloaded event to preload_end_y
        let last_preloaded_y = self
            .preloaded_events
            .last()
            .map(|ev| ev.position().clone())
            .unwrap_or_else(|| cur_y.clone());

        if last_preloaded_y < *preload_end_y {
            let new_events = self
                .all_events
                .events_in_y_range((Excluded(&last_preloaded_y), Included(preload_end_y)));
            self.preloaded_events.extend(new_events);
        }
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

    /// Compute visible events with their display ratios.
    ///
    /// Returns references to events to avoid unnecessary cloning.
    #[must_use]
    pub fn compute_visible_events(&self) -> Vec<(&PlayheadEvent, RangeInclusive<DisplayRatio>)> {
        let current_y = &self.progressed_y;
        let visible_window_y = self.visible_window_y();
        let scroll_factor = &self.current_scroll;

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
                let end_display_ratio = if let crate::chart_process::ChartEvent::Note {
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

                (event_with_pos, start_display_ratio..=end_display_ratio)
            })
            .collect()
    }

    /// Query events in a time range centered at current moment.
    #[must_use]
    pub fn events_in_time_range<R>(&self, range: R) -> Vec<PlayheadEvent>
    where
        R: std::ops::RangeBounds<TimeSpan>,
    {
        self.started_at.map_or_else(Vec::new, |started| {
            let last = self.last_poll_at.unwrap_or(started);
            // Calculate center time: elapsed time scaled by playback ratio
            let elapsed = last
                .checked_elapsed_since(started)
                .unwrap_or(TimeSpan::ZERO);
            let elapsed_nanos = elapsed.as_nanos().max(0) as u64;
            let elapsed_nanos = Decimal::from(elapsed_nanos);
            let center_nanos = (&elapsed_nanos * &self.playback_ratio)
                .to_u64()
                .unwrap_or(0);
            let center = TimeSpan::from_duration(Duration::from_nanos(center_nanos));
            self.all_events
                .events_in_time_range_offset_from(center, range)
        })
    }

    /// Handle control events.
    pub fn handle_control_event(&mut self, event: crate::chart_process::ControlEvent) {
        match event {
            crate::chart_process::ControlEvent::SetVisibleRangePerBpm {
                visible_range_per_bpm,
            } => {
                self.visible_range_per_bpm = visible_range_per_bpm;
            }
            crate::chart_process::ControlEvent::SetPlaybackRatio { ratio } => {
                self.playback_ratio = ratio;
                self.mark_velocity_dirty();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_process::base_bpm::BaseBpm;

    #[test]
    fn test_velocity_caching() {
        let mut core = ProcessorCore::new(
            Decimal::from(120),
            VisibleRangePerBpm::new(&BaseBpm::new(Decimal::from(120)), TimeSpan::SECOND),
            AllEventsIndex::new(std::collections::BTreeMap::new()),
            std::collections::BTreeMap::new(),
        );

        // First call computes velocity
        let v1 = core.calculate_velocity();
        assert!(v1 > Decimal::zero());

        // Second call should use cache
        let v2 = core.calculate_velocity();
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_flow_event_application() {
        let mut core = ProcessorCore::new(
            Decimal::from(120),
            VisibleRangePerBpm::new(&BaseBpm::new(Decimal::from(120)), TimeSpan::SECOND),
            AllEventsIndex::new(std::collections::BTreeMap::new()),
            std::collections::BTreeMap::new(),
        );

        let initial_bpm = core.current_bpm.clone();
        assert_eq!(initial_bpm, Decimal::from(120));

        // Apply BPM change
        let event = FlowEvent::Bpm(Decimal::from(180));
        event.apply_to(&mut core);

        assert_eq!(core.current_bpm, Decimal::from(180));
        assert!(core.velocity_dirty);
    }

    #[test]
    fn test_display_ratio_computation() {
        let current_y = YCoordinate::from(10.0);
        let event_y = YCoordinate::from(11.0);
        let visible_window_y = YCoordinate::from(2.0);
        let scroll_factor = Decimal::one();

        let ratio = ProcessorCore::compute_display_ratio(
            &event_y,
            &current_y,
            &visible_window_y,
            &scroll_factor,
        );

        // (11 - 10) / 2 = 0.5
        assert!((ratio.value().to_f64().unwrap() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_flow_event_priority_order() {
        // Test that events are sorted by priority: BPM > Speed > Scroll
        let mut events = vec![
            FlowEvent::Scroll(Decimal::from(2)),
            FlowEvent::Bpm(Decimal::from(180)),
            FlowEvent::Speed(Decimal::from(2)),
        ];

        FlowEvent::sort_by_priority(&mut events);

        // Verify order: BPM (0) > Speed (1) > Scroll (2)
        let Some(first) = events.first() else {
            panic!("events should not be empty");
        };
        let Some(second) = events.get(1) else {
            panic!("events should have at least 2 elements");
        };
        let Some(third) = events.get(2) else {
            panic!("events should have at least 3 elements");
        };

        assert!(matches!(first, FlowEvent::Bpm(_)));
        assert!(matches!(second, FlowEvent::Speed(_)));
        assert!(matches!(third, FlowEvent::Scroll(_)));
    }

    #[test]
    fn test_multiple_flow_events_at_same_position() {
        // Test that all events at the same position are applied
        let mut flow_events_by_y = BTreeMap::new();
        let y = YCoordinate::from(100.0);

        flow_events_by_y.insert(
            y.clone(),
            vec![
                FlowEvent::Bpm(Decimal::from(180)),
                FlowEvent::Speed(Decimal::from(2)),
            ],
        );

        let mut core = ProcessorCore::new(
            Decimal::from(120),
            VisibleRangePerBpm::new(&BaseBpm::new(Decimal::from(120)), TimeSpan::SECOND),
            AllEventsIndex::new(BTreeMap::new()),
            flow_events_by_y,
        );

        // Apply all events at this position
        let events = core.flow_events_by_y.get(&y).cloned().unwrap();
        let mut sorted_events = events;
        FlowEvent::sort_by_priority(&mut sorted_events);
        for evt in sorted_events {
            evt.apply_to(&mut core);
        }

        // Verify both events were applied
        assert_eq!(core.current_bpm, Decimal::from(180));
        assert_eq!(core.current_speed, Decimal::from(2));

        // Verify velocity is calculated correctly: (180 / 240) * 2.0 = 1.5
        let velocity = core.calculate_velocity();
        let expected = Decimal::from(180) / Decimal::from(240) * Decimal::from(2);
        assert_eq!(velocity, expected);
    }

    #[test]
    fn test_next_flow_events_after_returns_all() {
        // Test that next_flow_events_after returns all events at a position
        let mut flow_events_by_y = BTreeMap::new();
        let y1 = YCoordinate::from(100.0);
        let y2 = YCoordinate::from(200.0);

        flow_events_by_y.insert(
            y1.clone(),
            vec![
                FlowEvent::Bpm(Decimal::from(180)),
                FlowEvent::Speed(Decimal::from(2)),
            ],
        );
        flow_events_by_y.insert(y2, vec![FlowEvent::Scroll(Decimal::from(1))]);

        let core = ProcessorCore::new(
            Decimal::from(120),
            VisibleRangePerBpm::new(&BaseBpm::new(Decimal::from(120)), TimeSpan::SECOND),
            AllEventsIndex::new(BTreeMap::new()),
            flow_events_by_y,
        );

        // Get events after position 0
        let result = core.next_flow_events_after(&YCoordinate::zero());
        assert!(result.is_some());

        let (y, events) = result.unwrap();
        assert_eq!(y, y1);
        assert_eq!(events.len(), 2); // Should return both BPM and Speed events
    }
}
