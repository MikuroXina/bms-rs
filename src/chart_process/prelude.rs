//! Chart Process module prelude
//!
//! This module provides re-exports of commonly used types and traits from the chart_process module,
//! allowing users to import all needed items in one go.

// Re-export types
pub use super::types::{
    BaseBpm, BaseBpmGenerateStyle, BmpId, ChartEventId, ChartEventIdGenerator, DisplayRatio, WavId,
    YCoordinate,
};

// Re-export event types
pub use super::{ChartEvent, ControlEvent};

// Re-export trait
pub use super::ChartProcessor;

// Re-export common types from bms module
pub use crate::bms::prelude::{BgaLayer, Key, NoteKind, PlayerSide};

pub use crate::bms::prelude::SwBgaEvent;

// Re-export BmsProcessor from bms_processor module
pub use super::bms_processor::BmsProcessor;

// Re-export BmsonProcessor from bmson_processor module
#[cfg(feature = "bmson")]
pub use super::bmson_processor::BmsonProcessor;
