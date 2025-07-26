//! Random generator for parsing BMS format.
//!
//! [`RngMock`] can be used for testing the parser result with some random scopes.
//! [`RandRng`] can be used for generating random numbers with [`rand`] crate.
//!
//! [`rand`]: https://crates.io/crates/rand

use num::{BigUint, iter::RangeInclusive};

/// A random generator for parsing BMS.
pub trait Rng {
    /// Generates a random integer within the `range`. Returning the number outside the range will result weird.
    ///
    /// - Example:
    /// ```rust
    /// use bms_rs::bms::parse::rng::{Rng, RngMock};
    /// use num::BigUint;
    ///
    /// let mut rng = RngMock([BigUint::from(1u64)]);
    /// let n = rng.generate(num::range_inclusive(BigUint::from(1u64), BigUint::from(10u64)));
    /// assert!(n >= BigUint::from(1u64) && n <= BigUint::from(10u64));
    /// ```
    fn generate(&mut self, range: RangeInclusive<BigUint>) -> BigUint;
}

/// A random generator for mocking. This generates the number from the array in rotation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RngMock<const N: usize>(pub [BigUint; N]);

impl<const N: usize> Rng for RngMock<N> {
    fn generate(&mut self, _range: RangeInclusive<BigUint>) -> BigUint {
        self.0.rotate_left(1);
        self.0[N - 1].clone()
    }
}

/// A random generator for using [`rand`] crate.
///
/// [`rand`]: https://crates.io/crates/rand
/// - Example:
/// ```rust
/// use bms_rs::bms::parse::rng::{Rng, RandRng};
/// use rand::{rngs::StdRng, SeedableRng};
/// use num::BigUint;
///
/// let mut rng = RandRng(StdRng::seed_from_u64(42));
/// let n = rng.generate(num::range_inclusive(BigUint::from(1u64), BigUint::from(10u64)));
/// assert!(n >= BigUint::from(1u64) && n <= BigUint::from(10u64));
/// ```
#[cfg(feature = "rand")]
pub struct RandRng<R: rand::RngCore>(pub R);

#[cfg(feature = "rand")]
impl<R: rand::RngCore> Rng for RandRng<R> {
    fn generate(&mut self, range: RangeInclusive<BigUint>) -> BigUint {
        use core::ops::{Bound, RangeBounds};
        use num::One;

        let (Bound::Included(start), Bound::Included(end)) =
            (range.start_bound(), range.end_bound())
        else {
            unreachable!()
        };
        let width = end - start + BigUint::one();
        let width_bits = width.bits() as usize;

        loop {
            let mut bytes = vec![0u8; (width_bits + 7) / 8];
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
        let range = num::range_inclusive(start.clone(), end.clone());
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
