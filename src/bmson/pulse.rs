//! Pulse definition for bmson format. It represents only beat on the score, so you need to know previous BPMs for finding happening seconds of a note.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    bms::Decimal,
    parse::notes::Notes,
    time::{ObjTime, Track},
};

/// Note position for the chart [`super::Bmson`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PulseNumber(pub u64);

impl PulseNumber {
    /// Calculates an absolute difference of two pulses.
    pub const fn abs_diff(self, other: Self) -> u64 {
        self.0.abs_diff(other.0)
    }
}

/// Converter from [`ObjTime`] into pulses, which split one quarter note evenly.
#[derive(Debug, Clone)]
pub struct PulseConverter {
    pulses_at_track_start: BTreeMap<Track, u64>,
    resolution: u64,
}

impl PulseConverter {
    /// Creates a new converter from [`Notes`].
    pub fn new(notes: &Notes) -> Self {
        let resolution: u64 = notes.resolution_for_pulses();
        let last_track = notes.last_obj_time().map_or(0, |time| time.track.0);

        let mut pulses_at_track_start = BTreeMap::new();
        pulses_at_track_start.insert(Track(0), 0);
        let mut current_track: u64 = 0;
        let mut current_pulses: u64 = 0;
        loop {
            let section_len: f64 = notes
                .section_len_changes()
                .get(&Track(current_track))
                .map_or(Decimal::from(1u64), |section| section.length.clone())
                .try_into()
                .unwrap_or(1.0);
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
    pub fn get_pulses_at(&self, time: ObjTime) -> PulseNumber {
        let PulseNumber(track_base) = self.get_pulses_on(time.track);
        PulseNumber(track_base + (4 * self.resolution * time.numerator / time.denominator))
    }
}

#[test]
fn pulse_conversion() {
    use crate::parse::notes::SectionLenChangeObj;

    // Source BMS:
    // ```
    // #00102:0.75
    // #00103:1.25
    // ```
    let notes = {
        let mut notes = Notes::default();
        notes.push_section_len_change(SectionLenChangeObj {
            track: Track(1),
            length: Decimal::from(0.75),
        });
        notes.push_section_len_change(SectionLenChangeObj {
            track: Track(2),
            length: Decimal::from(1.25),
        });
        notes
    };
    let converter = PulseConverter::new(&notes);

    assert_eq!(converter.resolution, 1);

    assert_eq!(
        converter
            .get_pulses_at(ObjTime {
                track: Track(0),
                numerator: 0,
                denominator: 4
            })
            .0,
        0
    );
    assert_eq!(
        converter
            .get_pulses_at(ObjTime {
                track: Track(0),
                numerator: 2,
                denominator: 4
            })
            .0,
        2
    );
    assert_eq!(
        converter
            .get_pulses_at(ObjTime {
                track: Track(1),
                numerator: 0,
                denominator: 4
            })
            .0,
        4
    );
    assert_eq!(
        converter
            .get_pulses_at(ObjTime {
                track: Track(1),
                numerator: 2,
                denominator: 4
            })
            .0,
        6
    );
    assert_eq!(
        converter
            .get_pulses_at(ObjTime {
                track: Track(2),
                numerator: 0,
                denominator: 4
            })
            .0,
        7
    );
    assert_eq!(
        converter
            .get_pulses_at(ObjTime {
                track: Track(2),
                numerator: 2,
                denominator: 4
            })
            .0,
        9
    );
    assert_eq!(
        converter
            .get_pulses_at(ObjTime {
                track: Track(3),
                numerator: 0,
                denominator: 4
            })
            .0,
        12
    );
}
