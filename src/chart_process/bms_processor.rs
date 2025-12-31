//! Bms Processor Module.

use std::collections::HashMap;
use std::path::Path;

use gametime::{TimeSpan, TimeStamp};

use crate::bms::Decimal;
use crate::bms::prelude::*;
use crate::chart_process::parser::{BmsParser, ChartParser};
use crate::chart_process::player::{ChartPlayer, UniversalChartPlayer};
use crate::chart_process::resource::HashMapResourceMapping;
use crate::chart_process::types::{BmpId, DisplayRatio, VisibleRangePerBpm, WavId};
use crate::chart_process::{ChartProcessor, ControlEvent};

/// `ChartProcessor` of Bms files.
pub struct BmsProcessor {
    /// Universal chart player (handles all playback logic)
    player: UniversalChartPlayer<HashMapResourceMapping>,
}

impl BmsProcessor {
    /// Create processor with visible range per BPM configuration
    #[must_use]
    pub fn new<T: KeyLayoutMapper>(bms: &Bms, visible_range_per_bpm: VisibleRangePerBpm) -> Self {
        // Parse the BMS chart
        let parser = BmsParser::<T>::new(bms);
        let parse_output = parser.parse();

        // Create player from parse output
        let player = UniversalChartPlayer::from_parse_output(parse_output, visible_range_per_bpm);

        Self { player }
    }
}

impl ChartProcessor for BmsProcessor {
    fn audio_files(&self) -> HashMap<WavId, &Path> {
        self.player.audio_files()
    }

    fn bmp_files(&self) -> HashMap<BmpId, &Path> {
        self.player.bmp_files()
    }

    fn visible_range_per_bpm(&self) -> &VisibleRangePerBpm {
        self.player.visible_range_per_bpm()
    }

    fn current_bpm(&self) -> &Decimal {
        self.player.current_bpm()
    }

    fn current_speed(&self) -> &Decimal {
        self.player.current_speed()
    }

    fn current_scroll(&self) -> &Decimal {
        self.player.current_scroll()
    }

    fn playback_ratio(&self) -> &Decimal {
        self.player.playback_ratio()
    }

    fn start_play(&mut self, now: TimeStamp) {
        self.player.start_play(now);
    }

    fn started_at(&self) -> Option<TimeStamp> {
        self.player.started_at()
    }

    fn update(
        &mut self,
        now: TimeStamp,
    ) -> impl Iterator<Item = crate::chart_process::types::PlayheadEvent> {
        self.player.update(now)
    }

    fn events_in_time_range(
        &mut self,
        range: impl std::ops::RangeBounds<TimeSpan>,
    ) -> impl Iterator<Item = crate::chart_process::types::PlayheadEvent> {
        self.player.events_in_time_range(range)
    }

    fn post_events(&mut self, events: impl Iterator<Item = ControlEvent>) {
        self.player.post_events(events);
    }

    fn visible_events(
        &mut self,
    ) -> impl Iterator<
        Item = (
            crate::chart_process::types::PlayheadEvent,
            std::ops::RangeInclusive<DisplayRatio>,
        ),
    > {
        self.player.visible_events()
    }
}
