//! Module for base BPM generation strategies and types.

use strict_num_extended::PositiveF64;

/// Base BPM wrapper type.
///
/// Represents a positive BPM value used to derive default visible window length.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BaseBpm(pub PositiveF64);

impl BaseBpm {
    /// Create a new `BaseBpm` from `PositiveF64`.
    #[must_use]
    pub const fn new(value: PositiveF64) -> Self {
        Self(value)
    }

    /// Get the internal `PositiveF64` value.
    #[must_use]
    pub const fn value(&self) -> &PositiveF64 {
        &self.0
    }

    /// Convert to f64.
    #[must_use]
    pub const fn as_f64(&self) -> f64 {
        self.0.as_f64()
    }
}

impl From<PositiveF64> for BaseBpm {
    fn from(value: PositiveF64) -> Self {
        Self(value)
    }
}

impl From<BaseBpm> for PositiveF64 {
    fn from(value: BaseBpm) -> Self {
        value.0
    }
}

impl AsRef<PositiveF64> for BaseBpm {
    fn as_ref(&self) -> &PositiveF64 {
        &self.0
    }
}

impl Default for BaseBpm {
    fn default() -> Self {
        Self(PositiveF64::new_const(120.0))
    }
}

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
