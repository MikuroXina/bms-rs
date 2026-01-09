//! Chart Process module prelude
//!
//! This module provides re-exports of commonly used types and traits from the `chart_process` module,
//! allowing users to import all needed items in one go.

// Re-export types
pub use super::types::{
    BaseBpm, BaseBpmGenerator, BmpId, ChartEventId, ChartEventIdGenerator, ChartResources,
    DisplayRatio, FlowEvent, ManualBpmGenerator, MaxBpmGenerator, MinBpmGenerator, ParsedChart,
    StartBpmGenerator, VisibleRangePerBpm, WavId, YCoordinate,
};

// Re-export event types
pub use super::{ChartEvent, ControlEvent};

// Re-export common types from bms module
pub use crate::bms::prelude::{BgaLayer, Key, NoteKind, PlayerSide};

pub use crate::bms::prelude::SwBgaEvent;

// Re-export BmsProcessor from bms_processor module
pub use super::bms_processor::BmsProcessor;

// Re-export BmsonProcessor from bmson_processor module
#[cfg(feature = "bmson")]
pub use super::bmson_processor::BmsonProcessor;

// Re-export ChartPlayer
pub use super::player::{ChartPlayer, PlaybackState};
