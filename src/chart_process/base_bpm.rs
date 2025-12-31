//! Base BPM generator traits and implementations.

use std::time::Duration;

use gametime::TimeSpan;
use num::{ToPrimitive, Zero};

use crate::bms::{Decimal, model::Bms};
#[cfg(feature = "bmson")]
use crate::bmson::Bmson;

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
    ) -> super::YCoordinate {
        let speed_factor = current_speed * playback_ratio;
        let adjusted = current_bpm * self.value() * speed_factor / Decimal::from(240u64);
        super::YCoordinate::new(adjusted)
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

/// Trait for generating the base BPM used to derive default visible window length.
pub trait BaseBpmGenerator<S> {
    /// Generate a `BaseBpm` from the given source.
    /// Returns `None` when the source lacks sufficient information to determine a base BPM.
    fn generate(&self, source: &S) -> Option<BaseBpm>;
}

/// Generator that uses the chart's start/initial BPM.
#[derive(Debug, Clone, Copy, Default)]
pub struct StartBpmGenerator;

/// Generator that uses the minimum BPM across initial BPM and all BPM change events.
#[derive(Debug, Clone, Copy, Default)]
pub struct MinBpmGenerator;

/// Generator that uses the maximum BPM across initial BPM and all BPM change events.
#[derive(Debug, Clone, Copy, Default)]
pub struct MaxBpmGenerator;

/// Generator that uses a manually specified BPM value.
#[derive(Debug, Clone)]
pub struct ManualBpmGenerator(pub Decimal);

impl AsRef<Decimal> for ManualBpmGenerator {
    fn as_ref(&self) -> &Decimal {
        &self.0
    }
}

impl From<Decimal> for ManualBpmGenerator {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<ManualBpmGenerator> for Decimal {
    fn from(value: ManualBpmGenerator) -> Self {
        value.0
    }
}

impl ManualBpmGenerator {
    /// Returns a reference to the contained BPM value.
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }

    /// Consumes self and returns the contained BPM value.
    #[must_use]
    pub fn into_value(self) -> Decimal {
        self.0
    }
}

/// Base BPM wrapper type, encapsulating a `Decimal` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseBpm(pub Decimal);

impl AsRef<Decimal> for BaseBpm {
    fn as_ref(&self) -> &Decimal {
        &self.0
    }
}

impl BaseBpm {
    /// Create a new `BaseBpm`
    #[must_use]
    pub const fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Returns a reference to the contained BPM value.
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }

    /// Consumes self and returns the contained BPM value.
    #[must_use]
    pub fn into_value(self) -> Decimal {
        self.0
    }
}

impl From<Decimal> for BaseBpm {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<BaseBpm> for Decimal {
    fn from(value: BaseBpm) -> Self {
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
