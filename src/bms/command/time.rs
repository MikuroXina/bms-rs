//! Definitions of time in BMS.

use num::Integer;

/// A track, or measure, or bar, in the score. It must greater than 0, but some scores may include the 0 track, where the object is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Track(pub u64);

/// A time of the object on the score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjTime {
    /// The track, or measure, where the object is in.
    pub track: Track,
    /// The time offset numerator in the track.
    pub numerator: u64,
    /// The time offset denominator in the track.
    pub denominator: u64,
}

impl ObjTime {
    /// Create a new time.
    pub fn new(mut track: u64, mut numerator: u64, mut denominator: u64) -> Self {
        // If denominator is 0, set numerator to 0 and denominator to 1, and return.
        if denominator == 0 {
            return Self {
                track: Track(track),
                numerator: 0,
                denominator: 1,
            };
        }
        // If numerator is greater than denominator, add the integer part of numerator / denominator to track and set numerator to the remainder.
        if numerator > denominator {
            track += numerator / denominator;
            numerator %= denominator;
        }
        // If numerator is 0, set numerator to 0 and denominator to 1, and return.
        if numerator == 0 {
            return Self {
                track: Track(track),
                numerator: 0,
                denominator: 1,
            };
        }
        // Reduce the fraction to the simplest form.
        let gcd = numerator.gcd(&denominator);
        numerator /= gcd;
        denominator /= gcd;
        Self {
            track: Track(track),
            numerator,
            denominator,
        }
    }
}

impl PartialOrd for ObjTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ObjTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_time_in_track = self.numerator * other.denominator;
        let other_time_in_track = other.numerator * self.denominator;
        self.track
            .cmp(&other.track)
            .then(self_time_in_track.cmp(&other_time_in_track))
    }
}
