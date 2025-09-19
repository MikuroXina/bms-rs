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

use num::{BigUint, ToPrimitive};

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
    fn generate(&mut self, range: RangeInclusive<BigUint>) -> BigUint {
        use num::One;

        let (start, end) = (range.start(), range.end());
        let width = end - start + BigUint::one();

        // If the range is small enough to fit in i32, use the efficient next_int_bound method
        if let (Some(_start_i32), Some(width_i32)) =
            (start.to_i32(), width.to_i32().filter(|&w| w > 0))
        {
            let offset = self.next_int_bound(width_i32);
            return start + BigUint::from(offset as u32);
        }

        // For larger ranges, we need to generate multiple random values and combine them
        // This is a simplified approach for larger BigUint ranges
        let width_bits = width.bits() as usize;
        let width_clone = width.clone();
        let mut result = BigUint::ZERO;

        // Generate random bits until we have enough to cover the range
        let mut bits_generated = 0;
        while bits_generated < width_bits {
            let random_int = self.next_int();
            let random_bits = random_int.unsigned_abs();

            // Add these bits to our result
            let shift_amount = bits_generated.min(32);
            result |= BigUint::from(random_bits) << shift_amount;
            bits_generated += 32;

            // If we've exceeded the range, we need to reduce it
            if result >= width_clone {
                result %= width_clone.clone();
                break;
            }
        }

        // Ensure result is within width
        if result >= width_clone {
            result %= width_clone;
        }

        start + result
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
