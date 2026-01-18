//! Tests for `bms_rs::chart_process`.
//!
//! Unified time precision evaluation criterion:
//! - All time-related assertions must have absolute error less than 1 microsecond (0.000001s)
//! - Use [`assert_time_close`] function for unified time precision evaluation

pub mod bms_processor;
pub mod bmson_processor;
pub mod edge_cases;
pub mod update_consistency;

/// Unified time precision evaluation constant: 1 microsecond (unit: seconds)
pub(super) const MICROSECOND_EPSILON: f64 = 1e-6;

/// Assert that two floating-point time values are equal within 1 microsecond error margin
///
/// # Parameters
/// - `expected`: Expected value
/// - `actual`: Actual value
/// - `msg`: Error message description
///
/// # Assertion condition
/// `(expected - actual).abs() < 1e-6`
///
/// # Example
/// ```ignore
/// assert_time_close(1.5, actual, "activate_time");
/// ```
#[track_caller]
fn assert_time_close<T: Into<f64> + Copy>(expected: T, actual: T, msg: &str) {
    let expected = expected.into();
    let actual = actual.into();
    let diff = (expected - actual).abs();
    assert!(
        diff < MICROSECOND_EPSILON,
        "{msg}: expected {expected:.6}s, got {actual:.6}s, diff {diff:.9}s (allowed: {MICROSECOND_EPSILON}s)",
    );
}
