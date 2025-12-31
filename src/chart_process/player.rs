//! Chart player module
//!
//! Provides traits and implementations for playing pre-parsed chart events.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::path::Path;

use gametime::{TimeSpan, TimeStamp};

use crate::bms::Decimal;
use crate::chart_process::base_bpm::VisibleRangePerBpm;

use super::ControlEvent;
use super::EventParseOutput;
use super::core::{PlayheadEvent, ProcessorCore};
use super::resource::ResourceMapping;
use super::resource::{BmpId, WavId};
use super::{DisplayRatio, YCoordinate};

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
        flow_events_by_y: BTreeMap<YCoordinate, Vec<super::core::FlowEvent>>,
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
        self.core.started_at = None;
        self.core.last_poll_at = None;
        self.core.progressed_y = YCoordinate::zero();
        self.core.preloaded_events.clear();
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
    /// - The event with its position
    /// - The display ratio range (start..=end) for rendering
    ///
    /// For normal notes and events, start and end are the same.
    /// For long notes, the range represents the note's length.
    pub fn visible_events(
        &self,
    ) -> impl Iterator<Item = (PlayheadEvent, RangeInclusive<DisplayRatio>)> {
        self.core.compute_visible_events().into_iter()
    }
}
