//! Random generator for parsing BMS format.
//!
//! [`RngMock`] can be used for testing the parser result with some random scopes.

use std::ops::RangeInclusive;

/// A random generator for parsing BMS.
pub trait Rng {
    /// Generates a random integer within the `range`. Returning the number outside the range will result weird.
    fn gen(&mut self, range: RangeInclusive<u32>) -> u32;
}

/// A random generator for mocking. This generates the number from the array in rotation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RngMock<const N: usize>(pub [u32; N]);

impl<const N: usize> Rng for RngMock<N> {
    fn gen(&mut self, _range: std::ops::RangeInclusive<u32>) -> u32 {
        self.0.rotate_left(1);
        self.0[N - 1]
    }
}
