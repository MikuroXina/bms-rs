//! Mathematical utility functions for BMS parsing.

/// Calculates the greatest common divisor of two numbers using Euclid's algorithm.
#[must_use]
pub const fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Calculates the least common multiple of two numbers.
#[must_use]
pub const fn lcm(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        0
    } else {
        a / gcd(a, b) * b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(48, 18), 6);
        assert_eq!(gcd(18, 48), 6);
        assert_eq!(gcd(0, 5), 5);
        assert_eq!(gcd(5, 0), 5);
        assert_eq!(gcd(17, 17), 17);
        assert_eq!(gcd(1, 1), 1);
    }

    #[test]
    fn test_lcm() {
        assert_eq!(lcm(4, 6), 12);
        assert_eq!(lcm(6, 4), 12);
        assert_eq!(lcm(0, 5), 0);
        assert_eq!(lcm(5, 0), 0);
        assert_eq!(lcm(1, 1), 1);
        assert_eq!(lcm(21, 6), 42);
    }
}
