//! Utility functions for chart processing.

use crate::bms::Decimal;
use crate::chart_process::{BaseBpm, PointerSpeed, VisibleRangePerBpm, YCoordinate};
use std::time::Duration;

/// Compute visible window length in y units based on current BPM, base BPM, and reaction time.
/// Formula: (current_bpm / base_bpm) * reaction_time_seconds (derived from `Duration`).
#[must_use]
pub fn compute_visible_window_y(
    current_bpm: &Decimal,
    base_bpm: &BaseBpm,
    reaction_time: Duration,
) -> Decimal {
    let seconds = Decimal::from(reaction_time.as_secs_f64());
    (current_bpm.clone() / base_bpm.value().clone()) * seconds
}

/// Compute visible window length in y units based on current BPM and visible range per BPM.
/// Formula: current_bpm * visible_range_per_bpm
#[must_use]
pub fn compute_visible_window_y_from_visible_range(
    current_bpm: &Decimal,
    visible_range_per_bpm: &VisibleRangePerBpm,
) -> Decimal {
    current_bpm.clone() * visible_range_per_bpm.value().clone()
}

/// Compute pointer velocity in y units per second based on current BPM and pointer speed.
/// Formula: current_bpm * pointer_speed
#[must_use]
pub fn compute_pointer_velocity(current_bpm: &Decimal, pointer_speed: &PointerSpeed) -> Decimal {
    current_bpm.clone() * pointer_speed.value().clone()
}

/// Compute default visible y length (YCoordinate) from the selected base BPM and reaction time.
#[must_use]
pub fn compute_default_visible_y_length(
    base_bpm: &BaseBpm,
    reaction_time: Duration,
) -> YCoordinate {
    YCoordinate::from(compute_visible_window_y(
        base_bpm.value(),
        base_bpm,
        reaction_time,
    ))
}

/// Compute default visible y length (YCoordinate) from the selected visible range per BPM.
#[must_use]
pub fn compute_default_visible_y_length_from_visible_range(
    visible_range_per_bpm: &VisibleRangePerBpm,
) -> YCoordinate {
    // For default visible length, we use 1 BPM as reference
    YCoordinate::from(visible_range_per_bpm.value().clone())
}
