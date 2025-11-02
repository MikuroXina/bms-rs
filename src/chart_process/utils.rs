//! Utility functions for chart processing.

use crate::bms::Decimal;
use crate::chart_process::YCoordinate;

/// Compute visible window length in y units based on current BPM, base BPM, and reaction time.
/// Formula: (current_bpm / base_bpm) * reaction_time_seconds.
#[must_use]
pub fn compute_visible_window_y(
    current_bpm: Decimal,
    base_bpm: Decimal,
    reaction_time_seconds: Decimal,
) -> Decimal {
    (current_bpm / base_bpm) * reaction_time_seconds
}

/// Compute default visible y length (YCoordinate) from the selected base BPM and reaction time.
#[must_use]
pub fn compute_default_visible_y_length(
    base_bpm: Decimal,
    reaction_time_seconds: Decimal,
) -> YCoordinate {
    // When current BPM equals base BPM, visible window equals reaction time
    YCoordinate::from(compute_visible_window_y(
        base_bpm.clone(),
        base_bpm,
        reaction_time_seconds,
    ))
}
