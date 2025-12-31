//! Chart Process module prelude
//!
//! This module provides re-exports of commonly used types and traits from the `chart_process` module,
//! allowing users to import all needed items in one go.

// Re-export types from resource module
pub use super::resource::{BmpId, WavId};

// Re-export types from parent module
pub use super::{
    AllEventsIndex, ChartEventId, ChartEventIdGenerator, DisplayRatio, PlayheadEvent, YCoordinate,
};

// Re-export base BPM logic
pub use super::base_bpm::{
    BaseBpm, BaseBpmGenerator, ManualBpmGenerator, MaxBpmGenerator, MinBpmGenerator,
    StartBpmGenerator, VisibleRangePerBpm,
};

// Re-export event types
pub use super::{ChartEvent, ControlEvent};

// Re-export traits
pub use super::ChartProcessor;

// Re-export resource mapping
pub use super::resource::{HashMapResourceMapping, NameBasedResourceMapping, ResourceMapping};

// Re-export Y calculator
pub use super::y_calculator::{BmsYCalculator, create_bmson_y_calculator};

// Re-export processor types
pub use super::EventParseOutput;

// Re-export player types
pub use super::player::UniversalChartPlayer;

// Re-export common types from bms module
pub use crate::bms::prelude::{BgaLayer, Key, NoteKind, PlayerSide};

pub use crate::bms::prelude::SwBgaEvent;

// Re-export BmsProcessor from bms_processor module
pub use super::bms_processor::BmsProcessor;

// Re-export BmsonProcessor from bmson_processor module
#[cfg(feature = "bmson")]
pub use super::bmson_processor::BmsonProcessor;
