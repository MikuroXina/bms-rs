//! Chart parser module
//!
//! Provides traits and implementations for parsing different chart formats
//! into a unified representation of `PlayheadEvent` lists.

use std::collections::BTreeMap;

use crate::bms::Decimal;

use super::core::FlowEvent;
use super::resource::ResourceMapping;
use super::types::{AllEventsIndex, YCoordinate};

/// Output of chart parsing.
///
/// Contains all the information needed for chart playback.
pub struct EventParseOutput {
    /// All events with their positions and activation times
    pub all_events: AllEventsIndex,

    /// Flow events (BPM/Speed/Scroll changes) indexed by Y coordinate
    pub flow_events_by_y: BTreeMap<YCoordinate, Vec<FlowEvent>>,

    /// Initial BPM
    pub init_bpm: Decimal,

    /// Resource mapping
    pub resources: Box<dyn ResourceMapping>,
}

/// Chart parser trait.
///
/// Defines the interface for parsing different chart formats
/// into a unified `EventParseOutput`.
pub trait ChartParser {
    /// Parse the chart and generate event list.
    ///
    /// Returns an `EventParseOutput` containing all events and metadata.
    fn parse(&self) -> EventParseOutput;
}
