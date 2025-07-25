//! Random generator for parsing BMS format.
//!
//! [`RngMock`] can be used for testing the parser result with some random scopes.

use num::BigUint;

/// A random generator for parsing BMS.
pub trait Rng {
    /// Generates a random integer within the `range`. Returning the number outside the range will result weird.
    fn generate(&mut self, min: BigUint, max: BigUint) -> BigUint;
}

/// A random generator for mocking. This generates the number from the array in rotation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RngMock<const N: usize>(pub [BigUint; N]);

impl<const N: usize> Rng for RngMock<N> {
    fn generate(&mut self, _min: BigUint, _max: BigUint) -> BigUint {
        self.0.rotate_left(1);
        self.0[N - 1].clone()
    }
}
