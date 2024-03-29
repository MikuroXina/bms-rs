//! Definitions of time in BMS.

/// A track, or measure, where the object is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Track(pub u32);

/// A time of the object on the score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjTime {
    /// The track, or measure, where the object is in.
    pub track: Track,
    /// The time offset numerator in the track.
    pub numerator: u32,
    /// The time offset denominator in the track.
    pub denominator: u32,
}

impl ObjTime {
    /// Create a new time.
    ///
    /// # Panics
    ///
    /// Panics if `denominator` is 0 or `numerator` is greater than or equal to `denominator`.
    pub fn new(track: u32, numerator: u32, denominator: u32) -> Self {
        if track == 0 {
            eprintln!("warning: track 000 detected");
        }
        assert!(0 < denominator);
        assert!(numerator < denominator);
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
