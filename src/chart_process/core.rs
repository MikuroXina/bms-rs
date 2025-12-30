//! Core processor logic shared between BMS and BMSON processors.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::time::Duration;

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive, Zero};

use crate::bms::Decimal;
use crate::chart_process::types::{
    AllEventsIndex, DisplayRatio, PlayheadEvent, VisibleRangePerBpm, YCoordinate,
};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

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
            FlowEvent::Speed(_s) => {
                // Speed is format-specific (BMS only)
                // Handled by the concrete processor implementation
            }
            FlowEvent::Scroll(s) => {
                core.current_scroll = s.clone();
                // Scroll doesn't affect velocity
            }
        }
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
        self.mark_velocity_dirty();
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

    /// Get the current Y position.
    #[must_use]
    pub const fn progressed_y(&self) -> &YCoordinate {
        &self.progressed_y
    }

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
        if self.current_bpm <= Decimal::zero() {
            Decimal::from(f64::EPSILON)
        } else {
            let denom = Decimal::from(240);
            let base = &self.current_bpm / &denom;
            let v1 = base * speed.clone();
            let v = &v1 * &self.playback_ratio;
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

    /// Advance time to `now`, performing segmented integration.
    ///
    /// This is the core time progression algorithm, shared between BMS and BMSON.
    pub fn step_to(&mut self, now: TimeStamp, speed: &Decimal) {
        let Some(started) = self.started_at else {
            return;
        };
        let last = self.last_poll_at.unwrap_or(started);
        if now <= last {
            return;
        }

        let mut remaining_time = now - last;
        let mut cur_vel = self.calculate_velocity(speed);
        let mut cur_y = self.progressed_y.clone();

        // Advance in segments until time slice is used up
        loop {
            let cur_y_now = cur_y.clone();
            let next_event = self.next_flow_event_after(&cur_y_now);

            if next_event.is_none()
                || cur_vel <= Decimal::zero()
                || remaining_time <= TimeSpan::ZERO
            {
                // Advance directly to the end
                cur_y = cur_y_now
                    + YCoordinate::new(
                        cur_vel * Decimal::from(remaining_time.as_nanos().max(0))
                            / NANOS_PER_SECOND,
                    );
                break;
            }

            let Some((event_y, evt)) = next_event else {
                cur_y = cur_y_now
                    + YCoordinate::new(
                        cur_vel * Decimal::from(remaining_time.as_nanos().max(0))
                            / NANOS_PER_SECOND,
                    );
                break;
            };

            if event_y <= cur_y_now {
                // Defense: avoid infinite loop if event position doesn't advance
                evt.apply_to(self);
                cur_vel = self.calculate_velocity(speed);
                cur_y = cur_y_now;
                continue;
            }

            // Time required to reach event
            let distance = event_y.clone() - cur_y_now.clone();
            if cur_vel > Decimal::zero() {
                let time_to_event_nanos = ((distance.value() / &cur_vel)
                    * Decimal::from(NANOS_PER_SECOND))
                .to_u64()
                .unwrap_or(0);
                let time_to_event =
                    TimeSpan::from_duration(Duration::from_nanos(time_to_event_nanos));

                if time_to_event <= remaining_time {
                    // First advance to event point
                    cur_y = event_y;
                    remaining_time -= time_to_event;
                    evt.apply_to(self);
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

        self.progressed_y = cur_y;
        self.last_poll_at = Some(now);
    }

    /// Get visible window length in Y units.
    #[must_use]
    pub fn visible_window_y(&self, speed: &Decimal) -> YCoordinate {
        self.visible_range_per_bpm
            .window_y(&self.current_bpm, speed, &self.playback_ratio)
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

        let cur_y = &self.progressed_y;
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
        if visible_window_y.value() > &Decimal::zero() {
            let ratio_value =
                (event_y - current_y).value() / visible_window_y.value() * scroll_factor.clone();
            DisplayRatio::from(ratio_value)
        } else {
            DisplayRatio::at_judgment_line()
        }
    }

    /// Compute visible events with their display ratios.
    #[must_use]
    pub fn compute_visible_events(
        &self,
        speed: &Decimal,
    ) -> Vec<(PlayheadEvent, RangeInclusive<DisplayRatio>)> {
        let current_y = &self.progressed_y;
        let visible_window_y = self.visible_window_y(speed);
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

                (
                    event_with_pos.clone(),
                    start_display_ratio..=end_display_ratio,
                )
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
    use crate::chart_process::types::BaseBpm;

    #[test]
    fn test_velocity_caching() {
        let mut core = ProcessorCore::new(
            Decimal::from(120),
            VisibleRangePerBpm::new(&BaseBpm::new(Decimal::from(120)), TimeSpan::SECOND),
            AllEventsIndex::new(std::collections::BTreeMap::new()),
            std::collections::BTreeMap::new(),
        );

        let speed = Decimal::one();

        // First call computes velocity
        let v1 = core.calculate_velocity(&speed);
        assert!(v1 > Decimal::zero());

        // Second call should use cache
        let v2 = core.calculate_velocity(&speed);
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
}
