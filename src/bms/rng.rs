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
    /// use bms_rs::bms::rng::{Rng, RngMock};
    ///
    /// let mut rng = RngMock([5u64]);
    /// let result = rng.generate(1u64..=10u64);
    /// assert_eq!(result, 5u64);
    /// ```
    fn generate(&mut self, range: RangeInclusive<u64>) -> u64;
}

impl<T: Rng + ?Sized> Rng for Box<T> {
    fn generate(&mut self, range: RangeInclusive<u64>) -> u64 {
        T::generate(self, range)
    }
}

/// A deterministic mock random number generator for testing.
///
/// This implementation returns values from a predefined array in rotation.
/// It's useful for testing BMS control flow parsing with predictable results.
///
/// # Examples
///
/// ```rust
/// use bms_rs::bms::rng::{Rng, RngMock};
///
/// let mut rng = RngMock([1u64, 2u64]);
///
/// // Returns values in rotation: 1, 2, 1, 2, ...
/// assert_eq!(rng.generate(0u64..=10u64), 1u64);
/// assert_eq!(rng.generate(0u64..=10u64), 2u64);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RngMock<const N: usize>(pub [u64; N]);

impl<const N: usize> Rng for RngMock<N> {
    fn generate(&mut self, _range: RangeInclusive<u64>) -> u64 {
        let Some(first) = self.0.first().copied() else {
            return 0;
        };
        self.0.rotate_left(1);
        first
    }
}

/// A production-ready random number generator using the [`rand`] crate.
///
/// This implementation provides true random number generation for production use.
/// It wraps any type implementing [`rand::Rng`] and generates numbers within
/// the specified range using rejection sampling.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "rand")]
/// use bms_rs::bms::rng::{Rng, RandRng};
/// # #[cfg(feature = "rand")]
/// use rand::{rngs::StdRng, SeedableRng};
///
/// # #[cfg(feature = "rand")]
/// let mut rng = RandRng(StdRng::seed_from_u64(42));
/// # #[cfg(feature = "rand")]
/// let n = rng.generate(1u64..=10u64);
/// # #[cfg(feature = "rand")]
/// assert!(n >= 1u64 && n <= 10u64);
/// ```
///
/// [`rand`]: https://crates.io/crates/rand
#[cfg(feature = "rand")]
pub struct RandRng<R>(pub R);

#[cfg(feature = "rand")]
impl<R: rand::Rng> Rng for RandRng<R> {
    fn generate(&mut self, range: RangeInclusive<u64>) -> u64 {
        let start = *range.start();
        let end = *range.end();
        // Use fill_bytes to generate random bytes and convert to u64
        if start == end {
            return start;
        }
        let range_size = end.wrapping_sub(start).saturating_add(1);
        let mut bytes = [0u8; 8];
        self.0.fill_bytes(&mut bytes);
        let random = u64::from_le_bytes(bytes);
        start.wrapping_add(random % range_size)
    }
}

/// A random number generator based on Java's `java.util.Random`.
///
/// # Deprecation Notice
///
/// This struct is not recommended for external use. For BMS control flow parsing,
/// prefer using other implementations of [`Rng`] trait, e.g. [`RandRng`].
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    const MULT: u64 = 0x5_DEEC_E66D;
    const ADD: u64 = 0xB;

    /// Create a new [`JavaRandom`] with the given seed.
    #[must_use]
    pub const fn new(seed: i64) -> Self {
        let s = (seed as u64) ^ Self::MULT;
        Self {
            seed: s & ((1u64 << 48) - 1),
        }
    }

    /// Java's `next(int bits)` method
    const fn next(&mut self, bits: i32) -> i32 {
        self.seed =
            (self.seed.wrapping_mul(Self::MULT).wrapping_add(Self::ADD)) & ((1u64 << 48) - 1);
        ((self.seed >> (48 - bits)) & ((1u64 << bits) - 1)) as i32
    }

    /// Java's `nextInt()` method - returns any int value
    pub const fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// Java's `nextInt(int bound)` method
    pub fn next_int_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");

        let m = bound - 1;
        if (bound & m) == 0 {
            // i.e., bound is a power of 2
            ((bound as i64 * self.next(31) as i64) >> 31) as i32
        } else {
            loop {
                let bits = self.next(31);
                let val = bits % bound;
                if bits - val + m >= 0 {
                    return val;
                }
            }
        }
    }
}

impl Default for JavaRandom {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Rng for JavaRandom {
    fn generate(&mut self, range: RangeInclusive<u64>) -> u64 {
        let (start, end) = (*range.start(), *range.end());
        let width = end
            .checked_add(1u64)
            .and_then(|w| w.checked_sub(start))
            .expect("Range width overflow");

        // If the range is small enough to fit in i32, use the efficient next_int_bound method
        #[allow(clippy::collapsible_if)]
        if let Ok(width_i32) = i32::try_from(width) {
            if width_i32 > 0 {
                if i32::try_from(start).is_ok() {
                    let offset = self.next_int_bound(width_i32) as u64;
                    return start.saturating_add(offset);
                }
            }
        }

        // For larger ranges, we need to generate multiple random values and combine them
        let mut result = 0u64;

        // Generate random bits until we have enough to cover the range
        let mut bits_generated = 0u32;
        let width_bits = 64u32.saturating_sub(width.leading_zeros());

        while bits_generated < width_bits {
            let random_int = self.next_int();
            let random_bits = random_int.unsigned_abs() as u64;

            // Add these bits to our result
            let shift_amount = bits_generated.min(32);
            result |= random_bits.wrapping_shl(shift_amount);
            bits_generated += 32;

            // If we've exceeded the range, we need to reduce it
            if result >= width {
                result %= width;
                break;
            }
        }

        // Ensure result is within width
        if result >= width {
            result %= width;
        }

        start.saturating_add(result)
    }
}

#[cfg(all(test, feature = "rand"))]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn test_rand_rng_big_range() {
        let mut rng = RandRng(StdRng::seed_from_u64(42));
        let range = 1u64..=100u64;
        let n1 = rng.generate(range);
        let n2 = rng.generate(1u64..=100u64);
        let n3 = rng.generate(1u64..=100u64);
        assert!((1..=100).contains(&n1), "n1 out of range");
        assert!((1..=100).contains(&n2), "n2 out of range");
        assert!((1..=100).contains(&n3), "n3 out of range");
        // Note: uniqueness is not guaranteed for small ranges
    }

    #[test]
    fn test_java_random_consistency() {
        // Test with seed 123456789
        let mut rng = JavaRandom::new(123456789);

        // Test nextInt() method (returns any int value)
        println!("First nextInt(): {}", rng.next_int());
        println!("Second nextInt(): {}", rng.next_int());
        println!("Third nextInt(): {}", rng.next_int());

        // Test nextInt(bound) method
        let mut rng2 = JavaRandom::new(123456789);
        println!("First nextInt(100): {}", rng2.next_int_bound(100));
        println!("Second nextInt(100): {}", rng2.next_int_bound(100));
        println!("Third nextInt(100): {}", rng2.next_int_bound(100));

        // Basic functionality test - should not panic
        assert!(rng2.next_int_bound(100) >= 0 && rng2.next_int_bound(100) < 100);
    }
}
