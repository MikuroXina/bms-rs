//! Type definition module

use std::time::Duration;

use num::{One, ToPrimitive, Zero};

pub use super::TimeSpan;
use crate::bms::prelude::Bms;
#[cfg(feature = "bmson")]
use crate::bmson::prelude::Bmson;
use crate::{
    bms::Decimal,
    chart_process::base_bpm::{
        BaseBpm, BaseBpmGenerator, ManualBpmGenerator, MaxBpmGenerator, MinBpmGenerator,
        StartBpmGenerator,
    },
};

/// Flow events that affect playback speed/scroll.
#[derive(Debug, Clone)]
pub enum FlowEvent {
    /// BPM change event.
    Bpm(Decimal),
    /// Speed factor change event (BMS only).
    Speed(Decimal),
    /// Scroll factor change event.
    Scroll(Decimal),
}

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Visible range per BPM, representing the relationship between BPM and visible Y range.
/// Formula: `visible_y_range` = `current_bpm` * `visible_range_per_bpm`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleRangePerBpm(Decimal);

impl AsRef<Decimal> for VisibleRangePerBpm {
    fn as_ref(&self) -> &Decimal {
        &self.0
    }
}

impl VisibleRangePerBpm {
    /// Create a new `VisibleRangePerBpm` from base BPM and reaction time
    /// Formula: `visible_range_per_bpm` = `reaction_time_seconds` * 240 / `base_bpm`
    #[must_use]
    pub fn new(base_bpm: &BaseBpm, reaction_time: TimeSpan) -> Self {
        if base_bpm.value().is_zero() {
            Self(Decimal::zero())
        } else {
            Self(
                Decimal::from(reaction_time.as_nanos().max(0)) / NANOS_PER_SECOND
                    * Decimal::from(240u64)
                    / base_bpm.value().clone(),
            )
        }
    }

    /// Returns a reference to the contained value.
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }

    /// Consumes self and returns the contained value.
    #[must_use]
    pub fn into_value(self) -> Decimal {
        self.0
    }

    /// Calculate visible window length in y units based on current BPM, speed, and playback ratio.
    /// Formula: `visible_window_y = current_bpm * visible_range_per_bpm * current_speed * playback_ratio / 240`
    /// This ensures events stay in visible window for exactly `reaction_time` duration.
    #[must_use]
    pub fn window_y(
        &self,
        current_bpm: &Decimal,
        current_speed: &Decimal,
        playback_ratio: &Decimal,
    ) -> YCoordinate {
        let speed_factor = current_speed * playback_ratio;
        let adjusted = current_bpm * self.value() * speed_factor / Decimal::from(240u64);
        YCoordinate::new(adjusted)
    }

    /// Calculate reaction time from visible range per BPM
    /// Formula: `reaction_time` = `visible_range_per_bpm` / `playhead_speed`
    /// where `playhead_speed` = 1/240 (Y/sec per BPM)
    #[must_use]
    pub fn to_reaction_time(&self) -> TimeSpan {
        if self.0.is_zero() {
            TimeSpan::ZERO
        } else {
            let base = &self.0 * &Decimal::from(240);
            let nanos = (&base * &Decimal::from(NANOS_PER_SECOND))
                .to_u64()
                .unwrap_or(0);
            TimeSpan::from_duration(Duration::from_nanos(nanos))
        }
    }

    /// Create from Decimal value (for internal use)
    #[must_use]
    pub(crate) const fn from_decimal(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<Decimal> for VisibleRangePerBpm {
    fn from(value: Decimal) -> Self {
        Self::from_decimal(value)
    }
}

impl From<VisibleRangePerBpm> for Decimal {
    fn from(value: VisibleRangePerBpm) -> Self {
        value.0
    }
}

// ---- Generators for BMS ----
impl BaseBpmGenerator<Bms> for StartBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm.bpm.as_ref().cloned().map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for MinBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .iter()
            .cloned()
            .chain(
                bms.bpm
                    .bpm_changes
                    .values()
                    .map(|change| change.bpm.clone()),
            )
            .min()
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for MaxBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .iter()
            .cloned()
            .chain(
                bms.bpm
                    .bpm_changes
                    .values()
                    .map(|change| change.bpm.clone()),
            )
            .max()
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for ManualBpmGenerator {
    fn generate(&self, _bms: &Bms) -> Option<BaseBpm> {
        Some(BaseBpm::new(self.0.clone()))
    }
}

// ---- Generators for BMSON ----
#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for StartBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        Some(BaseBpm::new(Decimal::from(bmson.info.init_bpm.as_f64())))
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for MinBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        std::iter::once(Decimal::from(bmson.info.init_bpm.as_f64()))
            .chain(
                bmson
                    .bpm_events
                    .iter()
                    .map(|ev| Decimal::from(ev.bpm.as_f64())),
            )
            .min()
            .map(BaseBpm::new)
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for MaxBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        std::iter::once(Decimal::from(bmson.info.init_bpm.as_f64()))
            .chain(
                bmson
                    .bpm_events
                    .iter()
                    .map(|ev| Decimal::from(ev.bpm.as_f64())),
            )
            .max()
            .map(BaseBpm::new)
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for ManualBpmGenerator {
    fn generate(&self, _bmson: &Bmson<'a>) -> Option<BaseBpm> {
        Some(BaseBpm::new(self.0.clone()))
    }
}

/// Y coordinate wrapper type, using arbitrary precision decimal numbers.
///
/// Unified y unit description: In default 4/4 time, one measure equals 1; BMS uses `#SECLEN` for linear conversion, BMSON normalizes via `pulses / (4*resolution)` to measure units.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YCoordinate(pub Decimal);

impl AsRef<Decimal> for YCoordinate {
    fn as_ref(&self) -> &Decimal {
        &self.0
    }
}

impl YCoordinate {
    /// Create a new `YCoordinate`
    #[must_use]
    pub const fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Returns a reference to the contained value.
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }

    /// Consumes self and returns the contained value.
    #[must_use]
    pub fn into_value(self) -> Decimal {
        self.0
    }

    /// Creates a zero of Y coordinate.
    #[must_use]
    pub fn zero() -> Self {
        Self(Decimal::zero())
    }
}

impl From<Decimal> for YCoordinate {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<YCoordinate> for Decimal {
    fn from(value: YCoordinate) -> Self {
        value.0
    }
}

impl From<f64> for YCoordinate {
    fn from(value: f64) -> Self {
        Self(Decimal::from(value))
    }
}

impl std::ops::Add for YCoordinate {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Add for &YCoordinate {
    type Output = YCoordinate;

    fn add(self, rhs: Self) -> Self::Output {
        YCoordinate(&self.0 + &rhs.0)
    }
}

impl std::ops::Sub for YCoordinate {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Sub for &YCoordinate {
    type Output = YCoordinate;

    fn sub(self, rhs: Self) -> Self::Output {
        YCoordinate(&self.0 - &rhs.0)
    }
}

impl std::ops::Mul for YCoordinate {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl std::ops::Div for YCoordinate {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}

impl std::ops::Div for &YCoordinate {
    type Output = YCoordinate;

    fn div(self, rhs: Self) -> Self::Output {
        YCoordinate(&self.0 / &rhs.0)
    }
}

impl Zero for YCoordinate {
    fn zero() -> Self {
        Self(Decimal::zero())
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

/// Display ratio wrapper type, representing the actual position of a note in the display area.
///
/// 0 is the judgment line, 1 is the position where the note generally starts to appear.
/// The value of this type is only affected by: current Y, Y visible range, and current Speed, Scroll values.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct DisplayRatio(pub Decimal);

impl AsRef<Decimal> for DisplayRatio {
    fn as_ref(&self) -> &Decimal {
        &self.0
    }
}

impl DisplayRatio {
    /// Create a new `DisplayRatio`
    #[must_use]
    pub const fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Returns a reference to the contained value.
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }

    /// Consumes self and returns the contained value.
    #[must_use]
    pub fn into_value(self) -> Decimal {
        self.0
    }

    /// Create a `DisplayRatio` representing the judgment line (value 0)
    #[must_use]
    pub fn at_judgment_line() -> Self {
        Self(Decimal::zero())
    }

    /// Create a `DisplayRatio` representing the position where note starts to appear (value 1)
    #[must_use]
    pub fn at_appearance() -> Self {
        Self(Decimal::one())
    }
}

impl From<Decimal> for DisplayRatio {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<DisplayRatio> for Decimal {
    fn from(value: DisplayRatio) -> Self {
        value.0
    }
}

impl From<f64> for DisplayRatio {
    fn from(value: f64) -> Self {
        Self(Decimal::from(value))
    }
}
