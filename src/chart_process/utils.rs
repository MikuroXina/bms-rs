//! Utility functions for chart processing.

use crate::bms::Decimal;
use crate::chart_process::{BaseBpm, YCoordinate};
use std::time::Duration;

/// Compute visible window length in y units based on current BPM, base BPM, and reaction time.
/// Formula: (current_bpm / base_bpm) * reaction_time_seconds (derived from `Duration`).
#[must_use]
pub fn compute_visible_window_y(
    current_bpm: Decimal,
    base_bpm: BaseBpm,
    reaction_time: Duration,
) -> Decimal {
    let seconds = Decimal::from(reaction_time.as_secs_f64());
    (current_bpm / base_bpm.value().clone()) * seconds
}

/// Compute default visible y length (YCoordinate) from the selected base BPM and reaction time.
#[must_use]
pub fn compute_default_visible_y_length(base_bpm: BaseBpm, reaction_time: Duration) -> YCoordinate {
    // When current BPM equals base BPM, visible window equals reaction time
    YCoordinate::from(compute_visible_window_y(
        base_bpm.value().clone(),
        base_bpm,
        reaction_time,
    ))
}
