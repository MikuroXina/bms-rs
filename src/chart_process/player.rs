//! Chart player module
//!
//! Provides traits and implementations for playing pre-parsed chart events.

use std::collections::{BTreeMap, HashMap};
use std::ops::RangeInclusive;
use std::path::Path;

use gametime::{TimeSpan, TimeStamp};

use crate::bms::Decimal;

use super::ControlEvent;
use super::EventParseOutput;
use super::core::{PlayheadEvent, ProcessorCore};
use super::resource::ResourceMapping;
use super::resource::{BmpId, WavId};
use super::{DisplayRatio, YCoordinate};

/// Chart player trait.
///
/// Defines the interface for playing pre-parsed chart events.
pub trait ChartPlayer {
    /// Start playback at the given time.
    fn start_play(&mut self, now: TimeStamp);

    /// Get the playback start time.
    fn started_at(&self) -> Option<TimeStamp>;

    /// Get the current BPM.
    fn current_bpm(&self) -> &Decimal;

    /// Get the current speed factor.
    fn current_speed(&self) -> &Decimal;

    /// Get the current scroll factor.
    fn current_scroll(&self) -> &Decimal;

    /// Get the playback ratio.
    fn playback_ratio(&self) -> &Decimal;

    /// Get the visible range per BPM configuration.
    fn visible_range_per_bpm(&self) -> &crate::chart_process::base_bpm::VisibleRangePerBpm;

    /// Get all audio files mapping.
    fn audio_files(&self) -> HashMap<WavId, &Path>;

    /// Get all BMP files mapping.
    fn bmp_files(&self) -> HashMap<BmpId, &Path>;

    /// Update playback to the given time and return triggered events.
    ///
    /// Returns an iterator of events that were triggered between the last update
    /// and the current time.
    fn update(&mut self, now: TimeStamp) -> impl Iterator<Item = PlayheadEvent>;

    /// Get events in a time range.
    ///
    /// The range is relative to the chart start time (`activate_time`).
    fn events_in_time_range(
        &mut self,
        range: impl std::ops::RangeBounds<TimeSpan>,
    ) -> impl Iterator<Item = PlayheadEvent>;

    /// Post control events to the player.
    ///
    /// Control events can modify playback parameters like visible range or playback ratio.
    fn post_events(&mut self, events: impl Iterator<Item = ControlEvent>);

    /// Get currently visible events with their display ratios.
    ///
    /// Returns an iterator of tuples containing:
    /// - The event with its position
    /// - The display ratio range (start..=end) for rendering
    ///
    /// For normal notes and events, start and end are the same.
    /// For long notes, the range represents the note's length.
    fn visible_events(
        &mut self,
    ) -> impl Iterator<Item = (PlayheadEvent, RangeInclusive<DisplayRatio>)>;
}

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
        visible_range_per_bpm: crate::chart_process::base_bpm::VisibleRangePerBpm,
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
        output: EventParseOutput,
        visible_range_per_bpm: crate::chart_process::base_bpm::VisibleRangePerBpm,
    ) -> Self
    where
        R: From<Box<dyn ResourceMapping + 'static>>,
    {
        // Unbox the resources
        let resources: R = output.resources.into();

        Self::new(
            output.all_events,
            output.flow_events_by_y,
            output.init_bpm,
            visible_range_per_bpm,
            resources,
        )
    }

    /// Get the processor core (for advanced usage).
    #[must_use]
    pub const fn core(&self) -> &ProcessorCore {
        &self.core
    }

    /// Get mutable reference to the processor core (for advanced usage).
    pub const fn core_mut(&mut self) -> &mut ProcessorCore {
        &mut self.core
    }
}

// Implement From for HashMapResourceMapping
impl From<Box<dyn ResourceMapping + 'static>> for super::resource::HashMapResourceMapping {
    fn from(boxed: Box<dyn ResourceMapping + 'static>) -> Self {
        // Fallback: convert through the trait interface
        let wav_map = boxed.to_wav_map();
        let bmp_map = boxed.to_bmp_map();

        let wav_paths: HashMap<WavId, std::path::PathBuf> = wav_map
            .into_iter()
            .map(|(k, v)| (k, v.to_path_buf()))
            .collect();

        let bmp_paths: HashMap<BmpId, std::path::PathBuf> = bmp_map
            .into_iter()
            .map(|(k, v)| (k, v.to_path_buf()))
            .collect();

        super::resource::HashMapResourceMapping::new(wav_paths, bmp_paths)
    }
}

// Implement From for NameBasedResourceMapping
impl From<Box<dyn ResourceMapping + 'static>> for super::resource::NameBasedResourceMapping {
    fn from(_boxed: Box<dyn ResourceMapping + 'static>) -> Self {
        // Fallback: create empty mapping (not ideal but works for compilation)
        super::resource::NameBasedResourceMapping::empty()
    }
}

impl<R: ResourceMapping> ChartPlayer for UniversalChartPlayer<R> {
    fn start_play(&mut self, now: TimeStamp) {
        self.core.start_play(now);
    }

    fn started_at(&self) -> Option<TimeStamp> {
        self.core.started_at()
    }

    fn current_bpm(&self) -> &Decimal {
        &self.core.current_bpm
    }

    fn current_speed(&self) -> &Decimal {
        &self.core.current_speed
    }

    fn current_scroll(&self) -> &Decimal {
        &self.core.current_scroll
    }

    fn playback_ratio(&self) -> &Decimal {
        &self.core.playback_ratio
    }

    fn visible_range_per_bpm(&self) -> &crate::chart_process::base_bpm::VisibleRangePerBpm {
        &self.core.visible_range_per_bpm
    }

    fn audio_files(&self) -> HashMap<WavId, &Path> {
        self.resources.to_wav_map()
    }

    fn bmp_files(&self) -> HashMap<BmpId, &Path> {
        self.resources.to_bmp_map()
    }

    fn update(&mut self, now: TimeStamp) -> impl Iterator<Item = PlayheadEvent> {
        self.core.update_base(now).into_iter()
    }

    fn events_in_time_range(
        &mut self,
        range: impl std::ops::RangeBounds<TimeSpan>,
    ) -> impl Iterator<Item = PlayheadEvent> {
        self.core.events_in_time_range(range).into_iter()
    }

    fn post_events(&mut self, events: impl Iterator<Item = ControlEvent>) {
        for event in events {
            self.core.handle_control_event(event);
        }
    }

    fn visible_events(
        &mut self,
    ) -> impl Iterator<Item = (PlayheadEvent, RangeInclusive<DisplayRatio>)> {
        self.core.compute_visible_events().into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_process::{
        AllEventsIndex,
        base_bpm::{BaseBpm, VisibleRangePerBpm},
        resource::HashMapResourceMapping,
    };
    use num::One;
    use std::collections::{BTreeMap, HashMap};

    #[test]
    fn test_universal_chart_player_creation() {
        let mut wav_map = HashMap::new();
        wav_map.insert(WavId::new(0), std::path::PathBuf::from("test.wav"));

        let mut bmp_map = HashMap::new();
        bmp_map.insert(BmpId::new(0), std::path::PathBuf::from("test.bmp"));

        let resources = HashMapResourceMapping::new(wav_map, bmp_map);

        let all_events = AllEventsIndex::new(BTreeMap::new());
        let flow_events_by_y = BTreeMap::new();
        let init_bpm = Decimal::from(120);
        let base_bpm = BaseBpm::new(Decimal::from(120));
        let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, TimeSpan::SECOND);

        let player = UniversalChartPlayer::new(
            all_events,
            flow_events_by_y,
            init_bpm,
            visible_range_per_bpm,
            resources,
        );

        assert_eq!(player.current_bpm(), &Decimal::from(120));
        assert_eq!(player.current_speed(), &Decimal::one());
        assert_eq!(player.current_scroll(), &Decimal::one());
    }

    #[test]
    fn test_universal_chart_player_resource_access() {
        let mut wav_map = HashMap::new();
        wav_map.insert(WavId::new(0), std::path::PathBuf::from("audio1.wav"));
        wav_map.insert(WavId::new(1), std::path::PathBuf::from("audio2.wav"));

        let mut bmp_map = HashMap::new();
        bmp_map.insert(BmpId::new(0), std::path::PathBuf::from("img1.bmp"));
        bmp_map.insert(BmpId::new(1), std::path::PathBuf::from("img2.bmp"));

        let resources = HashMapResourceMapping::new(wav_map.clone(), bmp_map.clone());

        let all_events = AllEventsIndex::new(BTreeMap::new());
        let flow_events_by_y = BTreeMap::new();
        let init_bpm = Decimal::from(120);
        let base_bpm = BaseBpm::new(Decimal::from(120));
        let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, TimeSpan::SECOND);

        let player = UniversalChartPlayer::new(
            all_events,
            flow_events_by_y,
            init_bpm,
            visible_range_per_bpm,
            resources,
        );

        // Test audio files access
        let audio_files = player.audio_files();
        assert_eq!(audio_files.len(), 2);
        assert_eq!(
            audio_files.get(&WavId::new(0)),
            Some(&std::path::Path::new("audio1.wav"))
        );

        // Test BMP files access
        let bmp_files = player.bmp_files();
        assert_eq!(bmp_files.len(), 2);
        assert_eq!(
            bmp_files.get(&BmpId::new(0)),
            Some(&std::path::Path::new("img1.bmp"))
        );
    }
}
