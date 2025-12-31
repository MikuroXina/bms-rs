//! Y-coordinate calculation module
//!
//! Provides utilities for calculating Y coordinates in chart timeline.
//! Y coordinates unify the timeline representation across different chart formats.

use std::collections::BTreeMap;

use crate::bms::Decimal;
use crate::bms::prelude::*;
use num::{One, Zero};

use super::types::YCoordinate;

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
    /// Y coordinates memoization by track, which modified its length
    y_by_track: BTreeMap<Track, Decimal>,

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
        let mut last_track = 0;
        let mut y = Decimal::zero();

        for (&track, section_len_change) in &bms.section_len.section_len_changes {
            let passed_sections = (track.0 - last_track).saturating_sub(1);
            y += Decimal::from(passed_sections);
            y += &section_len_change.length;
            y_by_track.insert(track, y.clone());
            last_track = track.0;
        }

        Self {
            y_by_track,
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
            if let Some((&last_track, last_y)) = self.y_by_track.range(..=&track).last() {
                let passed_sections = track.0 - last_track.0;
                &Decimal::from(passed_sections) + last_y
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

    #[test]
    fn test_bms_y_calculator_basic() {
        let bms = crate::bms::prelude::Bms::default();

        // Test basic case: no section changes or speed changes
        let calculator = BmsYCalculator::from_bms(&bms);

        let y0 = calculator.get_y(crate::bms::prelude::ObjTime::start_of(
            crate::bms::prelude::Track(0),
        ));
        assert_eq!(y0.value(), &Decimal::zero());

        let y1 = calculator.get_y(crate::bms::prelude::ObjTime::start_of(
            crate::bms::prelude::Track(1),
        ));
        assert_eq!(y1.value(), &Decimal::from(1));

        let y2 = calculator.get_y(crate::bms::prelude::ObjTime::start_of(
            crate::bms::prelude::Track(2),
        ));
        assert_eq!(y2.value(), &Decimal::from(2));
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
        assert_eq!(*y0.value(), Decimal::from(1) / Decimal::from(2));

        // Quarter of track 1: y = 1.25
        let time1 =
            crate::bms::prelude::ObjTime::new(crate::bms::prelude::Track(1).0, 1, 4).unwrap();
        let y1 = calculator.get_y(time1);
        assert_eq!(
            *y1.value(),
            Decimal::from(1) + Decimal::from(1) / Decimal::from(4)
        );
    }

    #[test]
    fn test_bmson_y_calculator() {
        // Resolution of 240 means 1 measure = 960 pulses (4 * 240)
        let calculator = create_bmson_y_calculator(240);

        // 0 pulses = 0 measures
        let y0 = calculator(0);
        assert_eq!(*y0.value(), Decimal::zero());

        // 960 pulses = 1 measure
        let y1 = calculator(960);
        assert_eq!(*y1.value(), Decimal::from(1));

        // 1920 pulses = 2 measures
        let y2 = calculator(1920);
        assert_eq!(*y2.value(), Decimal::from(2));

        // 480 pulses = 0.5 measures
        let y3 = calculator(480);
        assert_eq!(*y3.value(), Decimal::from(1) / Decimal::from(2));
    }
}
