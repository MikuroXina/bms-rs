//! Y-coordinate calculation module
//!
//! Provides utilities for calculating Y coordinates in chart timeline.
//! Y coordinates unify the timeline representation across different chart formats.

use std::collections::BTreeMap;

use crate::bms::Decimal;
use crate::bms::prelude::*;
use num::{One, Zero};

use super::YCoordinate;

/// BMS Y-coordinate calculator.
///
/// Computes Y coordinates for BMS format charts, taking into account:
/// - Section length changes
/// - Speed factor changes
/// - Track-based positioning
///
/// The Y coordinate is a unified timeline representation where one measure
/// equals 1 in default 4/4 time signature.
#[derive(Debug, Clone)]
pub struct BmsYCalculator {
    /// Y coordinates memoization by track
    y_by_track: BTreeMap<Track, Decimal>,

    /// Section lengths by track (needed to skip zero-length sections)
    section_lengths: BTreeMap<Track, Decimal>,

    /// Speed change events sorted by time
    speed_changes: BTreeMap<ObjTime, SpeedObj>,
}

impl BmsYCalculator {
    /// Create a new `BmsYCalculator` from a BMS chart.
    ///
    /// This preprocesses the BMS chart to build a memoization table
    /// for efficient Y coordinate calculation.
    #[must_use]
    pub fn from_bms(bms: &Bms) -> Self {
        let mut y_by_track: BTreeMap<Track, Decimal> = BTreeMap::new();
        let mut section_lengths: BTreeMap<Track, Decimal> = BTreeMap::new();
        let mut last_track = 0;
        let mut y = Decimal::zero();

        for (&track, section_len_change) in &bms.section_len.section_len_changes {
            let passed_sections = (track.0 - last_track).saturating_sub(1);
            y += Decimal::from(passed_sections);
            y += &section_len_change.length;

            y_by_track.insert(track, y.clone());
            section_lengths.insert(track, section_len_change.length.clone());
            last_track = track.0;
        }

        Self {
            y_by_track,
            section_lengths,
            speed_changes: bms.speed.speed_factor_changes.clone(),
        }
    }

    /// Get the Y coordinate at a given time in the BMS chart.
    ///
    /// This method efficiently computes the Y coordinate by:
    /// 1. Finding the section Y at the given track
    /// 2. Adding the fractional position within the track
    /// 3. Applying the speed factor at that time
    ///
    /// # Arguments
    /// * `time` - The time position in the BMS chart
    ///
    /// # Returns
    /// The Y coordinate at the given time
    #[must_use]
    pub fn get_y(&self, time: ObjTime) -> YCoordinate {
        let section_y = {
            let track = time.track();

            // Find the last record where Y actually increased (non-zero length)
            // For zero-length sections, Y stays the same as the previous section
            let last_entry = self.y_by_track.range(..=&track).last();

            if let Some((&last_track, last_y)) = last_entry {
                // Check if this record has zero length
                let has_zero_length = self
                    .section_lengths
                    .get(&last_track)
                    .is_some_and(|len| *len == Decimal::zero());

                if has_zero_length && *last_y == Decimal::zero() {
                    // All sections up to this point have zero length
                    // Use track number directly
                    Decimal::from(track.0)
                } else if has_zero_length {
                    // This is a zero-length section, but previous sections had non-zero length
                    // We need to skip it and find where Y started increasing
                    let mut prev_y = None;
                    for (&t, y) in self.y_by_track.range(..=&last_track) {
                        let len = self
                            .section_lengths
                            .get(&t)
                            .cloned()
                            .unwrap_or_else(Decimal::one);
                        if len > Decimal::zero() {
                            prev_y = Some(y);
                        }
                    }

                    prev_y.map_or_else(
                        || Decimal::from(track.0),
                        |prev| {
                            // Calculate from the last non-zero point
                            let passed = track.0 - last_track.0;
                            &Decimal::from(passed) + prev
                        },
                    )
                } else {
                    // Normal case: calculate from last record
                    let passed_sections = track.0 - last_track.0;
                    &Decimal::from(passed_sections) + last_y
                }
            } else {
                // there is no sections modified its length until
                Decimal::from(track.0)
            }
        };

        let fraction = if time.denominator().get() > 0 {
            Decimal::from(time.numerator()) / Decimal::from(time.denominator().get())
        } else {
            Default::default()
        };

        let factor = self
            .speed_changes
            .range(..=time)
            .last()
            .map_or_else(Decimal::one, |(_, obj)| obj.factor.clone());

        YCoordinate((section_y + fraction) * factor)
    }
}

/// Create a BMSON Y-coordinate calculator function.
///
/// BMSON uses a resolution-based system where Y coordinates are computed
/// by normalizing pulses to measure units.
///
/// # Arguments
/// * `resolution` - The resolution value from BMSON info
///
/// # Returns
/// A function that converts pulse values to Y coordinates
///
/// # Formula
/// `y = pulses / (4 * resolution)`
///
/// This normalizes the pulse values to measure units, where one measure
/// equals 1 in default 4/4 time signature.
pub fn create_bmson_y_calculator(resolution: u32) -> impl Fn(u64) -> YCoordinate + Clone {
    let pulses_denom = Decimal::from(4 * resolution);
    move |pulses: u64| YCoordinate::new(Decimal::from(pulses) / pulses_denom.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use num::Zero;
    use std::f64::EPSILON as F64_EPSILON;

    /// Assert that Y coordinate matches expected value with high precision (f64::EPSILON)
    fn assert_y_high_precision(actual: &YCoordinate, expected: impl Into<Decimal>, msg: &str) {
        let expected_val = expected.into();
        let f64_epsilon = Decimal::from(F64_EPSILON);

        // Check if values are close in high precision
        let diff = (actual.value() - &expected_val).abs();

        // For very small expected values, use absolute difference
        if expected_val.abs() < Decimal::from(1e-10) {
            assert!(
                diff <= f64_epsilon,
                "{}: Expected {}, got {}, absolute difference {} exceeds f64::EPSILON ({})",
                msg,
                expected_val,
                actual.value(),
                diff,
                f64_epsilon
            );
        } else {
            // For larger values, use relative difference
            let relative_diff = &diff / &expected_val.abs();
            assert!(
                relative_diff <= f64_epsilon,
                "{}: Expected {}, got {}, relative error {} exceeds f64::EPSILON ({})",
                msg,
                expected_val,
                actual.value(),
                relative_diff,
                f64_epsilon
            );
        }
    }

    /// Assert that two Decimal values are equal with high precision (f64::EPSILON)
    fn assert_decimal_eq_high_precision(actual: &Decimal, expected: &Decimal, msg: &str) {
        assert_y_high_precision(&YCoordinate(actual.clone()), expected.clone(), msg);
    }

    #[test]
    fn test_bms_y_calculator_basic() {
        let bms = crate::bms::prelude::Bms::default();

        // Test basic case: no section changes or speed changes
        let calculator = BmsYCalculator::from_bms(&bms);

        let y0 = calculator.get_y(crate::bms::prelude::ObjTime::start_of(
            crate::bms::prelude::Track(0),
        ));
        assert_y_high_precision(&y0, Decimal::zero(), "Y at track 0");

        let y1 = calculator.get_y(crate::bms::prelude::ObjTime::start_of(
            crate::bms::prelude::Track(1),
        ));
        assert_y_high_precision(&y1, Decimal::from(1), "Y at track 1");

        let y2 = calculator.get_y(crate::bms::prelude::ObjTime::start_of(
            crate::bms::prelude::Track(2),
        ));
        assert_y_high_precision(&y2, Decimal::from(2), "Y at track 2");
    }

    #[test]
    fn test_bms_y_calculator_fraction() {
        let bms = crate::bms::prelude::Bms::default();
        let calculator = BmsYCalculator::from_bms(&bms);

        // Test fractional positions within a track
        // Middle of track 0: y = 0.5
        let time0 =
            crate::bms::prelude::ObjTime::new(crate::bms::prelude::Track(0).0, 1, 2).unwrap();
        let y0 = calculator.get_y(time0);
        assert_y_high_precision(&y0, Decimal::from(1) / Decimal::from(2), "Y at middle of track 0");

        // Quarter of track 1: y = 1.25
        let time1 =
            crate::bms::prelude::ObjTime::new(crate::bms::prelude::Track(1).0, 1, 4).unwrap();
        let y1 = calculator.get_y(time1);
        assert_y_high_precision(
            &y1,
            Decimal::from(1) + Decimal::from(1) / Decimal::from(4),
            "Y at quarter of track 1",
        );
    }

    #[test]
    fn test_bmson_y_calculator() {
        // Resolution of 240 means 1 measure = 960 pulses (4 * 240)
        let calculator = create_bmson_y_calculator(240);

        // 0 pulses = 0 measures
        let y0 = calculator(0);
        assert_y_high_precision(&y0, Decimal::zero(), "Y at 0 pulses");

        // 960 pulses = 1 measure
        let y1 = calculator(960);
        assert_y_high_precision(&y1, Decimal::from(1), "Y at 960 pulses");

        // 1920 pulses = 2 measures
        let y2 = calculator(1920);
        assert_y_high_precision(&y2, Decimal::from(2), "Y at 1920 pulses");

        // 480 pulses = 0.5 measures
        let y3 = calculator(480);
        assert_y_high_precision(&y3, Decimal::from(1) / Decimal::from(2), "Y at 480 pulses");
    }

    // ========================================================================
    // Zero Length Measure Tests
    // ========================================================================

    #[test]
    fn test_consecutive_zero_length_measures() {
        // Test handling of multiple consecutive zero-length measures
        // Track 1-8 (measures 2-9) all have length 0
        let mut bms = crate::bms::prelude::Bms::default();
        for i in 1..=8 {
            bms.section_len.section_len_changes.insert(
                crate::bms::prelude::Track(i),
                crate::bms::model::obj::SectionLenChangeObj {
                    track: crate::bms::prelude::Track(i),
                    length: Decimal::zero(),
                },
            );
        }

        let calculator = BmsYCalculator::from_bms(&bms);

        // Track 0 (measure 1): y = 0
        let y0 = calculator.get_y(crate::bms::prelude::ObjTime::start_of(
            crate::bms::prelude::Track(0),
        ));
        assert_y_high_precision(&y0, Decimal::zero(), "Y at track 0");

        // Track 8 (measure 9): y = 8
        // Consecutive zero-length measures still have Y coordinates increasing by track number,
        // but they don't occupy length on the timeline
        let y8 = calculator.get_y(crate::bms::prelude::ObjTime::start_of(
            crate::bms::prelude::Track(8),
        ));
        assert_y_high_precision(&y8, Decimal::from(8), "Y at track 8");

        // Track 9 (measure 10): y = 9
        let y9 = calculator.get_y(crate::bms::prelude::ObjTime::start_of(
            crate::bms::prelude::Track(9),
        ));
        assert_y_high_precision(&y9, Decimal::from(9), "Y at track 9");
    }

    #[test]
    fn test_zero_denominator_no_panic() {
        // Test handling when denominator is zero
        let bms = crate::bms::prelude::Bms::default();
        let calculator = BmsYCalculator::from_bms(&bms);

        // Try to construct an ObjTime with denominator 0
        // ObjTime::new validates the denominator, returning None if denominator is 0
        let time_invalid_result =
            crate::bms::prelude::ObjTime::new(crate::bms::prelude::Track(0).0, 1, 0);

        // Verify that ObjTime with denominator 0 cannot be constructed (this is expected behavior)
        assert!(
            time_invalid_result.is_none(),
            "ObjTime with denominator 0 should be invalid"
        );

        // Test denominator check logic in normal case
        // For valid ObjTime, get_y already checks denominator > 0
        let time_normal = crate::bms::prelude::ObjTime::start_of(crate::bms::prelude::Track(0));
        let y = calculator.get_y(time_normal);
        assert_y_high_precision(&y, Decimal::zero(), "Y at start of track 0");
    }
}
