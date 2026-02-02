//! Chart Process module prelude
//!
//! This module provides re-exports of commonly used types and traits from the `chart_process` module,
//! allowing users to import all needed items in one go.

// Re-export types
pub use super::base_bpm::{
    BaseBpm, BaseBpmGenerator, ManualBpmGenerator, MaxBpmGenerator, MinBpmGenerator,
    StartBpmGenerator,
};
pub use super::player::{DisplayRatio, VisibleRangePerBpm};
pub use super::processor::{
    AllEventsIndex, BmpId, ChartEventId, ChartEventIdGenerator, ChartResources, ParsedChart, WavId,
};
pub use super::{FlowEvent, YCoordinate};

// Re-export event types
pub use super::{ChartEvent, PlayheadEvent};

// Re-export common types from bms module
pub use crate::bms::prelude::{BgaLayer, Key, NoteKind, PlayerSide};

pub use crate::bms::prelude::SwBgaEvent;

// Re-export BmsProcessor from bms module
pub use super::processor::bms::BmsProcessor;

// Re-export BmsonProcessor from bmson module
#[cfg(feature = "bmson")]
pub use super::processor::bmson::BmsonProcessor;

// Re-export ChartPlayer
pub use super::player::{ChartPlayer, PlaybackState};
