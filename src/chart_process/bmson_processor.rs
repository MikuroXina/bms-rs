//! Bmson Processor Module.

use std::collections::HashMap;
use std::path::Path;

use gametime::{TimeSpan, TimeStamp};

use crate::bmson::prelude::*;
use crate::chart_process::parser::{BmsonParser, ChartParser};
use crate::chart_process::player::{ChartPlayer, UniversalChartPlayer};
use crate::chart_process::resource::NameBasedResourceMapping;
use crate::chart_process::types::{BmpId, DisplayRatio, VisibleRangePerBpm, WavId};
use crate::chart_process::{ChartProcessor, ControlEvent};

/// `ChartProcessor` of Bmson files.
pub struct BmsonProcessor {
    /// Universal chart player (handles all playback logic)
    player: UniversalChartPlayer<NameBasedResourceMapping>,
}

impl BmsonProcessor {
    /// Create BMSON processor with visible range per BPM configuration.
    #[must_use]
    pub fn new(bmson: &Bmson<'_>, visible_range_per_bpm: VisibleRangePerBpm) -> Self {
        // Parse the BMSON chart
        let parser = BmsonParser::new(bmson);
        let parse_output = parser.parse();

        // Create player from parse output
        let player = UniversalChartPlayer::from_parse_output(parse_output, visible_range_per_bpm);

        Self { player }
    }
}

impl ChartProcessor for BmsonProcessor {
    fn audio_files(&self) -> HashMap<WavId, &Path> {
        self.player.audio_files()
    }

    fn bmp_files(&self) -> HashMap<BmpId, &Path> {
        self.player.bmp_files()
    }

    fn visible_range_per_bpm(&self) -> &VisibleRangePerBpm {
        self.player.visible_range_per_bpm()
    }

    fn current_bpm(&self) -> &crate::bms::Decimal {
        self.player.current_bpm()
    }

    fn current_speed(&self) -> &crate::bms::Decimal {
        self.player.current_speed()
    }

    fn current_scroll(&self) -> &crate::bms::Decimal {
        self.player.current_scroll()
    }

    fn playback_ratio(&self) -> &crate::bms::Decimal {
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
