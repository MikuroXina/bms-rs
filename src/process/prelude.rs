//! Process module prelude
//!
//! This module provides re-exports of commonly used types and traits from the `process` module,
//! allowing users to import all needed items in one go.

// Re-export types from the process root (all types are now in the module root)
pub use super::{
    AllEventsIndex, BmpId, Chart, ChartEventId, ChartEventIdGenerator, ChartResources, Process,
    WavId, calculate_cumulative_times,
};
pub use super::event::{BmsEvent, ChartEvent, FlowEvent, PlayheadEvent, YCoordinate};
pub use gametime::TimeSpan;

// Re-export NonNegativeF64 for backward compatibility
pub use strict_num_extended::NonNegativeF64;
