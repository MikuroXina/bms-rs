//! Module for base BPM generation strategies and types.

use crate::bms::prelude::Bms;
#[cfg(feature = "bmson")]
use crate::bmson::prelude::Bmson;
use strict_num_extended::FinF64;

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
#[derive(Debug, Clone, Copy)]
pub struct ManualBpmGenerator(pub FinF64);

impl AsRef<FinF64> for ManualBpmGenerator {
    fn as_ref(&self) -> &FinF64 {
        &self.0
    }
}

impl From<FinF64> for ManualBpmGenerator {
    fn from(value: FinF64) -> Self {
        Self(value)
    }
}

impl From<ManualBpmGenerator> for FinF64 {
    fn from(value: ManualBpmGenerator) -> Self {
        value.0
    }
}

impl ManualBpmGenerator {
    /// Returns a reference to the contained BPM value.
    #[must_use]
    pub const fn value(&self) -> &FinF64 {
        &self.0
    }

    /// Consumes self and returns the contained BPM value.
    #[must_use]
    pub const fn into_value(self) -> FinF64 {
        self.0
    }
}

/// Base BPM wrapper type, encapsulating a `FinF64` value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaseBpm(pub FinF64);

impl AsRef<FinF64> for BaseBpm {
    fn as_ref(&self) -> &FinF64 {
        &self.0
    }
}

impl BaseBpm {
    /// Create a new `BaseBpm`
    #[must_use]
    pub const fn new(value: FinF64) -> Self {
        Self(value)
    }

    /// Returns a reference to the contained BPM value.
    #[must_use]
    pub const fn value(&self) -> &FinF64 {
        &self.0
    }

    /// Consumes self and returns the contained BPM value.
    #[must_use]
    pub const fn into_value(self) -> FinF64 {
        self.0
    }
}

impl From<FinF64> for BaseBpm {
    fn from(value: FinF64) -> Self {
        Self(value)
    }
}

impl From<BaseBpm> for FinF64 {
    fn from(value: BaseBpm) -> Self {
        value.0
    }
}

// ---- Generators for BMS ----
impl BaseBpmGenerator<Bms> for StartBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm.bpm.as_ref().map(|bpm| {
            BaseBpm::new(
                *bpm.value()
                    .as_ref()
                    .expect("parsed BPM value should be valid"),
            )
        })
    }
}

impl BaseBpmGenerator<Bms> for MinBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .iter()
            .map(|bpm| {
                *bpm.value()
                    .as_ref()
                    .expect("parsed BPM value should be valid")
            })
            .chain(bms.bpm.bpm_changes.values().map(|change| change.bpm))
            .min()
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for MaxBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .iter()
            .map(|bpm| {
                *bpm.value()
                    .as_ref()
                    .expect("parsed BPM value should be valid")
            })
            .chain(bms.bpm.bpm_changes.values().map(|change| change.bpm))
            .max()
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for ManualBpmGenerator {
    fn generate(&self, _bms: &Bms) -> Option<BaseBpm> {
        Some(BaseBpm::new(self.0))
    }
}

// ---- Generators for BMSON ----
#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for StartBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        FinF64::new(bmson.info.init_bpm.as_f64())
            .ok()
            .map(BaseBpm::new)
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for MinBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        std::iter::once(bmson.info.init_bpm)
            .chain(bmson.bpm_events.iter().map(|ev| ev.bpm))
            .min()
            .map(|bpm| BaseBpm::new(FinF64::new(bpm.as_f64()).expect("BPM should be finite")))
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for MaxBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        std::iter::once(bmson.info.init_bpm)
            .chain(bmson.bpm_events.iter().map(|ev| ev.bpm))
            .max()
            .map(|bpm| BaseBpm::new(FinF64::new(bpm.as_f64()).expect("BPM should be finite")))
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for ManualBpmGenerator {
    fn generate(&self, _bmson: &Bmson<'a>) -> Option<BaseBpm> {
        Some(BaseBpm::new(self.0))
    }
}
