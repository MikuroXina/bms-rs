//! Pulse definition for bmson format. It represents only beat on the score, so you need to know previous BPMs for finding happening seconds of a note.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bms::command::time::{ObjTime, Track};
use crate::bmson::prelude::FinF64;

/// Note position for the chart [`super::Bmson`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PulseNumber(pub u64);

impl PulseNumber {
    /// Calculates an absolute difference of two pulses.
    #[must_use]
    pub const fn abs_diff(self, other: Self) -> u64 {
        self.0.abs_diff(other.0)
    }

    /// Returns the contained pulse number value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Converter from [`ObjTime`] into pulses, which split one quarter note evenly.
#[derive(Debug, Clone)]
pub struct PulseConverter {
    pulses_at_track_start: BTreeMap<Track, u64>,
    resolution: u64,
}

impl PulseConverter {
    /// Creates a new converter from [`crate::bms::model::Bms`].
    #[must_use]
    pub fn new(bms: &crate::bms::model::Bms) -> Self {
        let resolution = bms.resolution_for_pulses();
        let last_track = bms.last_obj_time().map_or(0, |time| time.track().0);

        let mut pulses_at_track_start = BTreeMap::new();
        pulses_at_track_start.insert(Track(0), 0);
        let mut current_track: u64 = 0;
        let mut current_pulses: u64 = 0;
        loop {
            let section_len: f64 = bms
                .section_len
                .section_len_changes
                .get(&Track(current_track))
                .map_or_else(
                    || FinF64::new(1.0).expect("1 should be finite"),
                    |section| section.length,
                )
                .into();
            current_pulses = current_pulses.saturating_add((section_len * 4.0) as u64 * resolution);
            current_track += 1;
            pulses_at_track_start.insert(Track(current_track), current_pulses);
            if last_track < current_track {
                break;
            }
        }
        Self {
            pulses_at_track_start,
            resolution,
        }
    }

    /// Gets pulses on the start of [`Track`].
    #[must_use]
    pub fn get_pulses_on(&self, track: Track) -> PulseNumber {
        PulseNumber(
            self.pulses_at_track_start
                .get(&track)
                .copied()
                .or_else(|| {
                    self.pulses_at_track_start
                        .last_key_value()
                        .map(|(_, &pulses)| pulses)
                })
                .unwrap_or_default(),
        )
    }

    /// Gets pulses at the [`ObjTime`].
    #[must_use]
    pub fn get_pulses_at(&self, time: ObjTime) -> PulseNumber {
        let PulseNumber(track_base) = self.get_pulses_on(time.track());
        PulseNumber(
            track_base + (4 * self.resolution * time.numerator() / time.denominator().get()),
        )
    }
}

#[test]
fn pulse_conversion() {
    use crate::bms::model::{obj::SectionLenChangeObj, section_len::SectionLenObjects};

    // Source BMS:
    // ```
    // #00102:0.75
    // #00103:1.25
    // ```
    let notes = {
        let mut notes = SectionLenObjects::default();
        let prompt_handler = crate::bms::parse::prompt::AlwaysUseNewer;
        notes
            .push_section_len_change(
                SectionLenChangeObj {
                    track: Track(1),
                    length: FinF64::new(0.75).expect("0.75 should be finite"),
                },
                &prompt_handler,
            )
            .expect("NonZeroU64::new should succeed for non-zero values");
        notes
            .push_section_len_change(
                SectionLenChangeObj {
                    track: Track(2),
                    length: FinF64::new(1.25).expect("1.25 should be finite"),
                },
                &prompt_handler,
            )
            .expect("NonZeroU64::new should succeed for non-zero values");
        notes
    };
    let converter = PulseConverter::new(&crate::bms::model::Bms {
        section_len: notes,
        ..Default::default()
    });

    assert_eq!(converter.resolution, 1);

    assert_eq!(converter.get_pulses_at(ObjTime::start_of(0.into())).0, 0);
    assert_eq!(
        converter
            .get_pulses_at(ObjTime::new(0, 2, 4).expect("4 should be a valid denominator"))
            .0,
        2
    );
    assert_eq!(converter.get_pulses_at(ObjTime::start_of(1.into())).0, 4);
    assert_eq!(
        converter
            .get_pulses_at(ObjTime::new(1, 2, 4).expect("4 should be a valid denominator"))
            .0,
        6
    );
    assert_eq!(converter.get_pulses_at(ObjTime::start_of(2.into())).0, 7);
    assert_eq!(
        converter
            .get_pulses_at(ObjTime::new(2, 2, 4).expect("4 should be a valid denominator"))
            .0,
        9
    );
    assert_eq!(converter.get_pulses_at(ObjTime::start_of(3.into())).0, 12);
}
