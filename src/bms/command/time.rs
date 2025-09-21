//! Definitions of time in BMS.

use num::Integer;
use std::num::NonZeroU64;

/// A track, or measure, or bar, in the score. It must greater than 0, but some scores may include the 0 track, where the object is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Track(pub u64);

impl std::fmt::Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Track: {:03}", self.0)
    }
}

/// A time of the object on the score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjTime {
    /// The track, or measure, where the object is in.
    pub track: Track,
    /// The time offset numerator in the track.
    pub numerator: u64,
    /// The time offset denominator in the track.
    pub denominator: NonZeroU64,
}

impl ObjTime {
    /// Create a new time.
    #[must_use]
    pub fn new(track: u64, numerator: u64, denominator: NonZeroU64) -> Self {
        // If numerator is greater than denominator, add the integer part of numerator / denominator to track and set numerator to the remainder.
        let (track, numerator) = if numerator > denominator.get() {
            (
                track + (numerator / denominator.get()),
                numerator % denominator.get(),
            )
        } else {
            (track, numerator)
        };
        // Reduce the fraction to the simplest form.
        // Note: 0.gcd(&num) == num, when num > 0
        let gcd = numerator.gcd(&denominator.get());
        Self {
            track: Track(track),
            numerator: numerator / gcd,
            denominator: NonZeroU64::new(denominator.get() / gcd)
                .expect("GCD should never make denominator zero"),
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
        let self_time_in_track = self.numerator * other.denominator.get();
        let other_time_in_track = other.numerator * self.denominator.get();
        self.track
            .cmp(&other.track)
            .then(self_time_in_track.cmp(&other_time_in_track))
    }
}

impl std::fmt::Display for ObjTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ObjTime: {}, {} / {}",
            self.track,
            self.numerator,
            self.denominator.get()
        )
    }
}
