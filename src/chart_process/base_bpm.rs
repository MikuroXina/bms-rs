//! Module for base BPM generation strategies and types.

use crate::bms::prelude::Bms;
#[cfg(feature = "bmson")]
use crate::bmson::prelude::Bmson;
use crate::chart_process::BaseBpm;
use strict_num_extended::PositiveF64;

/// Trait for generating the base BPM used to derive default visible window length.
pub trait BaseBpmGenerator<S> {
    /// Generate a base BPM value from the given source.
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
pub struct ManualBpmGenerator(pub BaseBpm);

impl AsRef<BaseBpm> for ManualBpmGenerator {
    fn as_ref(&self) -> &BaseBpm {
        &self.0
    }
}

impl AsRef<PositiveF64> for ManualBpmGenerator {
    fn as_ref(&self) -> &PositiveF64 {
        &self.0.0
    }
}

impl From<BaseBpm> for ManualBpmGenerator {
    fn from(value: BaseBpm) -> Self {
        Self(value)
    }
}

impl From<ManualBpmGenerator> for BaseBpm {
    fn from(value: ManualBpmGenerator) -> Self {
        value.0
    }
}

impl From<PositiveF64> for ManualBpmGenerator {
    fn from(value: PositiveF64) -> Self {
        Self(BaseBpm::new(value))
    }
}

impl From<ManualBpmGenerator> for PositiveF64 {
    fn from(value: ManualBpmGenerator) -> Self {
        value.0.0
    }
}

impl ManualBpmGenerator {
    /// Returns a reference to the contained BPM value.
    #[must_use]
    pub const fn value(&self) -> &BaseBpm {
        &self.0
    }

    /// Consumes self and returns the contained BPM value.
    #[must_use]
    pub const fn into_value(self) -> BaseBpm {
        self.0
    }
}

// ---- Generators for BMS ----
impl BaseBpmGenerator<Bms> for StartBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .as_ref()
            .and_then(|bpm| bpm.value().as_ref().ok().copied())
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for MinBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .iter()
            .filter_map(|bpm| bpm.value().as_ref().ok().copied())
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
            .filter_map(|bpm| bpm.value().as_ref().ok().copied())
            .chain(bms.bpm.bpm_changes.values().map(|change| change.bpm))
            .max()
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for ManualBpmGenerator {
    fn generate(&self, _bms: &Bms) -> Option<BaseBpm> {
        Some(self.0)
    }
}

// ---- Generators for BMSON ----
#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for StartBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        Some(BaseBpm::new(bmson.info.init_bpm))
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for MinBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        std::iter::once(bmson.info.init_bpm)
            .chain(bmson.bpm_events.iter().map(|ev| ev.bpm))
            .min()
            .map(BaseBpm::new)
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for MaxBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        std::iter::once(bmson.info.init_bpm)
            .chain(bmson.bpm_events.iter().map(|ev| ev.bpm))
            .max()
            .map(BaseBpm::new)
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for ManualBpmGenerator {
    fn generate(&self, _bmson: &Bmson<'a>) -> Option<BaseBpm> {
        Some(self.0)
    }
}
