//! Random number generation for BMS control flow parsing.
//!
//! This module provides the [`Rng`] trait and implementations for generating random numbers
//! used in BMS control flow constructs like `#RANDOM` and `#SWITCH` blocks.
//!
//! # Overview
//!
//! The random number generation is essential for:
//!
//! - **Random Blocks**: Selecting which `#IF` branch to execute based on random values
//! - **Switch Blocks**: Determining which `#CASE` branch to execute
//! - **Testing**: Providing deterministic behavior for reproducible test results
//!
//! # Implementations
//!
//! ## [`RngMock`]
//!
//! A deterministic mock implementation for testing that returns predefined values in rotation:
//!
//! ## [`RandRng`]
//!
//! A production-ready implementation using the [`rand`] crate for true random number generation:
//!
//! [`rand`]: https://crates.io/crates/rand

use core::ops::RangeInclusive;

use num::BigUint;

/// A random number generator for BMS control flow parsing.
///
/// This trait defines the interface for generating random numbers used in BMS control flow
/// constructs. Implementations should generate numbers within the specified range.
///
/// # Contract
///
/// - The generated number must be within the specified `range` (inclusive)
/// - Returning a number outside the range may cause undefined behavior in the parser
/// - The implementation should be deterministic for testing purposes when needed
pub trait Rng {
    /// Generates a random integer within the specified `range`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bms_rs::bms::ast::rng::{Rng, RngMock};
    /// use num::BigUint;
    ///
    /// let mut rng = RngMock([BigUint::from(5u64)]);
    /// let result = rng.generate(BigUint::from(1u64)..=BigUint::from(10u64));
    /// assert_eq!(result, BigUint::from(5u64));
    /// ```
    fn generate(&mut self, range: RangeInclusive<BigUint>) -> BigUint;
}

/// A deterministic mock random number generator for testing.
///
/// This implementation returns values from a predefined array in rotation.
/// It's useful for testing BMS control flow parsing with predictable results.
///
/// # Examples
///
/// ```rust
/// use bms_rs::bms::ast::rng::{Rng, RngMock};
/// use num::BigUint;
///
/// let mut rng = RngMock([BigUint::from(1u64), BigUint::from(2u64)]);
///
/// // Returns values in rotation: 1, 2, 1, 2, ...
/// assert_eq!(rng.generate(BigUint::from(0u64)..=BigUint::from(10u64)), BigUint::from(1u64));
/// assert_eq!(rng.generate(BigUint::from(0u64)..=BigUint::from(10u64)), BigUint::from(2u64));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RngMock<const N: usize>(pub [BigUint; N]);

impl<const N: usize> Rng for RngMock<N> {
    fn generate(&mut self, _range: RangeInclusive<BigUint>) -> BigUint {
        self.0.rotate_left(1);
        self.0[N - 1].clone()
    }
}

/// A production-ready random number generator using the [`rand`] crate.
///
/// This implementation provides true random number generation for production use.
/// It wraps any type implementing [`rand::RngCore`] and generates numbers within
/// the specified range using rejection sampling.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "rand")]
/// use bms_rs::bms::ast::rng::{Rng, RandRng};
/// # #[cfg(feature = "rand")]
/// use rand::{rngs::StdRng, SeedableRng};
/// # #[cfg(feature = "rand")]
/// use num::BigUint;
///
/// # #[cfg(feature = "rand")]
/// let mut rng = RandRng(StdRng::seed_from_u64(42));
/// # #[cfg(feature = "rand")]
/// let n = rng.generate(BigUint::from(1u64)..=BigUint::from(10u64));
/// # #[cfg(feature = "rand")]
/// assert!(n >= BigUint::from(1u64) && n <= BigUint::from(10u64));
/// ```
///
/// [`rand`]: https://crates.io/crates/rand
#[cfg(feature = "rand")]
pub struct RandRng<R>(pub R);

#[cfg(feature = "rand")]
impl<R: rand::RngCore> Rng for RandRng<R> {
    fn generate(&mut self, range: RangeInclusive<BigUint>) -> BigUint {
        use num::One;

        let (start, end) = (range.start(), range.end());
        let width = end - start + BigUint::one();
        let width_bits = width.bits() as usize;

        loop {
            let mut bytes = vec![0u8; width_bits.div_ceil(8)];
            self.0.fill_bytes(&mut bytes);
            let mut n = BigUint::from_bytes_le(&bytes);
            if n < width {
                n += start;
                return n;
            }
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod tests {
    use super::*;
    use num::BigUint;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn test_rand_rng_big_range() {
        let start = BigUint::parse_bytes(b"10000000000000000000000000000000000000000000000000", 10)
            .unwrap();
        let end = BigUint::parse_bytes(b"10000000000000000000000000000000000000000000000099", 10)
            .unwrap();
        let mut rng = RandRng(StdRng::seed_from_u64(42));
        let range = start.clone()..=end.clone();
        let n1 = rng.generate(range.clone());
        let n2 = rng.generate(range.clone());
        let n3 = rng.generate(range.clone());
        assert!(n1 >= start && n1 <= end, "n1 out of range");
        assert!(n2 >= start && n2 <= end, "n2 out of range");
        assert!(n3 >= start && n3 <= end, "n3 out of range");
        assert!(
            n1 != n2 && n1 != n3 && n2 != n3,
            "random numbers are not unique"
        );
    }
}
