//! Chart Player Module.
//!
//! Unified player for parsed charts, managing playback state and event processing.

use std::collections::HashMap;
use std::path::Path;

use gametime::{TimeSpan, TimeStamp};

use crate::bms::Decimal;
use crate::chart_process::core::ProcessorCore;
use crate::chart_process::types::{
    BmpId, DisplayRatio, ParsedChart, PlayheadEvent, VisibleRangePerBpm, WavId,
};
use crate::chart_process::{ChartEvent, ControlEvent};

/// Unified chart player.
///
/// This player takes a parsed chart and manages all playback state and event processing.
pub struct ChartPlayer {
    /// Parsed chart data (immutable).
    chart: ParsedChart,

    /// Core processor logic (mutable playback state).
    core: ProcessorCore,

    /// Current Speed factor (updated during playback).
    current_speed: Decimal,
}

impl ChartPlayer {
    /// Create a new player from a parsed chart.
    #[must_use]
    pub fn new(mut chart: ParsedChart, visible_range_per_bpm: VisibleRangePerBpm) -> Self {
        // Extract flow_events from chart (take ownership)
        let flow_events = std::mem::take(&mut chart.flow_events);

        let core = ProcessorCore::new(
            chart.init_bpm.clone(),
            visible_range_per_bpm,
            chart.events.clone(),
            flow_events,
        );

        let current_speed = chart.init_speed.clone();

        Self {
            chart,
            core,
            current_speed,
        }
    }

    // ===== Playback Control =====

    /// Start playback at the given time.
    pub fn start_play(&mut self, now: TimeStamp) {
        self.core.start_play(now);
        self.current_speed = self.chart.init_speed.clone();
    }

    /// Update playback to the given time, return triggered events.
    pub fn update(&mut self, now: TimeStamp) -> Vec<PlayheadEvent> {
        let prev_y = self.core.progressed_y().clone();
        self.core.step_to(now, &self.current_speed);
        let cur_y = self.core.progressed_y();

        // Calculate preload range: current y + visible y range
        let visible_y_length = self.core.visible_window_y(&self.current_speed);
        let preload_end_y = cur_y + &visible_y_length;

        use std::ops::Bound::{Excluded, Included};

        // Collect events triggered at current moment
        let mut triggered_events = self
            .core
            .events_in_y_range((Excluded(&prev_y), Included(cur_y)));

        self.core.update_preloaded_events(&preload_end_y);

        // Apply Speed changes
        for event in &triggered_events {
            if let ChartEvent::SpeedChange { factor } = event.event() {
                self.current_speed = factor.clone();
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
            self.core.handle_control_event(evt);
        }
    }

    // ===== State Query =====

    /// Get audio file resources (id to path mapping).
    #[must_use]
    pub fn audio_files(&self) -> HashMap<WavId, &Path> {
        self.chart
            .resources
            .wav_files
            .iter()
            .map(|(id, path)| (*id, path.as_path()))
            .collect()
    }

    /// Get BGA/BMP image resources (id to path mapping).
    #[must_use]
    pub fn bmp_files(&self) -> HashMap<BmpId, &Path> {
        self.chart
            .resources
            .bmp_files
            .iter()
            .map(|(id, path)| (*id, path.as_path()))
            .collect()
    }

    /// Get current BPM.
    #[must_use]
    pub const fn current_bpm(&self) -> &Decimal {
        &self.core.current_bpm
    }

    /// Get current Speed factor.
    #[must_use]
    pub const fn current_speed(&self) -> &Decimal {
        &self.current_speed
    }

    /// Get current Scroll factor.
    #[must_use]
    pub const fn current_scroll(&self) -> &Decimal {
        &self.core.current_scroll
    }

    /// Get current playback ratio.
    #[must_use]
    pub const fn playback_ratio(&self) -> &Decimal {
        &self.core.playback_ratio
    }

    /// Get visible range per BPM.
    #[must_use]
    pub const fn visible_range_per_bpm(&self) -> &VisibleRangePerBpm {
        &self.core.visible_range_per_bpm
    }

    /// Get playback start time.
    #[must_use]
    pub const fn started_at(&self) -> Option<TimeStamp> {
        self.core.started_at
    }

    // ===== Visible Events =====

    /// Get all events in current visible area (with display positions).
    pub fn visible_events(
        &mut self,
    ) -> Vec<(PlayheadEvent, std::ops::RangeInclusive<DisplayRatio>)> {
        self.core.compute_visible_events(&self.current_speed)
    }

    /// Query events in a time window.
    pub fn events_in_time_range(
        &self,
        range: impl std::ops::RangeBounds<TimeSpan>,
    ) -> Vec<PlayheadEvent> {
        self.core.events_in_time_range(range)
    }
}
