//! Bmson Processor Module.

use crate::bms::Decimal;
use crate::bmson::prelude::*;
use crate::chart_process::parser::{BmsonParser, ChartParser};
use crate::chart_process::player::UniversalChartPlayer;
use crate::chart_process::resource::{NameBasedResourceMapping, ResourceMapping};
use crate::chart_process::types::{AllEventsIndex, VisibleRangePerBpm};

/// `ChartProcessor` of Bmson files.
///
/// This processor parses BMSON charts and produces an `EventParseOutput`.
/// Use the `to_player()` method to convert the parse output into a playable chart.
pub struct BmsonProcessor {
    /// Parsed chart output
    output: crate::chart_process::parser::EventParseOutput,
}

impl BmsonProcessor {
    /// Create BMSON processor by parsing BMSON chart.
    #[must_use]
    pub fn new(bmson: &Bmson<'_>) -> Self {
        // Parse the BMSON chart
        let parser = BmsonParser::new(bmson);
        let output = parser.parse();

        Self { output }
    }

    /// Convert the parse output into a playable chart.
    ///
    /// # Arguments
    /// * `visible_range_per_bpm` - Visible range configuration for playback
    #[must_use]
    pub fn to_player(
        self,
        visible_range_per_bpm: VisibleRangePerBpm,
    ) -> UniversalChartPlayer<NameBasedResourceMapping> {
        UniversalChartPlayer::from_parse_output(self.output, visible_range_per_bpm)
    }

    /// Get access to all parsed events.
    #[must_use]
    pub const fn all_events(&self) -> &AllEventsIndex {
        &self.output.all_events
    }

    /// Get the initial BPM.
    #[must_use]
    pub const fn init_bpm(&self) -> &Decimal {
        &self.output.init_bpm
    }

    /// Get access to the resource mapping.
    #[must_use]
    pub fn resources(&self) -> &dyn ResourceMapping {
        self.output.resources.as_ref()
    }
}
