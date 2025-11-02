//! Utility functions for chart processing.

use std::str::FromStr;

use crate::bms::Decimal;
use crate::chart_process::YCoordinate;

/// Compute visible window length in y units based on current BPM.
/// Formula: (current_bpm / 120) * 0.6 seconds.
#[must_use]
pub fn compute_visible_window_y(current_bpm: Decimal) -> Decimal {
    let reaction_time_seconds = Decimal::from_str("0.6").unwrap(); // 600ms
    let base_bpm = Decimal::from(120);
    (current_bpm / base_bpm) * reaction_time_seconds
}

/// Compute default visible y length (YCoordinate) from the initial BPM.
#[must_use]
pub fn compute_default_visible_y_length(init_bpm: Decimal) -> YCoordinate {
    YCoordinate::from(compute_visible_window_y(init_bpm))
}
