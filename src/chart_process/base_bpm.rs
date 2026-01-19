//! Module for base BPM generation strategies and types.

use crate::bms::Decimal;
use crate::bms::prelude::Bms;
#[cfg(feature = "bmson")]
use crate::bmson::prelude::Bmson;

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
