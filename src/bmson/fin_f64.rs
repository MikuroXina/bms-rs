//! Finite binary64 definition.

use serde::{Deserialize, Serialize};

/// `f64` but it has only finite value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct FinF64(f64);

impl Eq for FinF64 {}
impl PartialOrd for FinF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for FinF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl From<FinF64> for f64 {
    fn from(value: FinF64) -> Self {
        value.as_f64()
    }
}

#[derive(Debug, thiserror::Error)]
pub struct TryFromFloatError(pub(crate) ());

impl std::fmt::Display for TryFromFloatError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "finite number expected")
    }
}

impl TryFrom<f64> for FinF64 {
    type Error = TryFromFloatError;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        value
            .is_finite()
            .then_some(Self(value))
            .ok_or(TryFromFloatError(()))
    }
}

impl AsRef<f64> for FinF64 {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

impl FinF64 {
    /// Creates a new `FinF64` from `f64` if `float` is finite, otherwise returns `None`.
    #[inline]
    pub fn new(float: f64) -> Option<Self> {
        float.try_from().ok()
    }

    /// Gets the internal value.
    #[inline]
    pub fn as_f64(self) -> f64 {
        self.0
    }
}
