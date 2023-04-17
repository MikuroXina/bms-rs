//! Pulse definition for bmson format. It represents only beat on the score, so you need to know previous BPMs for finding happening seconds of a note.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    parse::notes::Notes,
    time::{ObjTime, Track},
};

/// Note position for the chart [`Bmson`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PulseNumber(pub u32);

impl PulseNumber {
    /// Calculates an absolute difference of two pulses.
    pub const fn abs_diff(self, other: Self) -> u32 {
        self.0.abs_diff(other.0)
    }
}

/// Converter from [`ObjTime`] into pulses, which split one quarter note evenly.
#[derive(Debug, Clone)]
pub struct PulseConverter {
    pulses_at_track_start: BTreeMap<Track, u32>,
    resolution: u32,
}

impl PulseConverter {
    /// Creates a new converter from [`Notes`].
    pub fn new(notes: &Notes) -> Self {
        let resolution = notes.resolution_for_pulses();
        let mut pulses_at_track_start = BTreeMap::new();
        let mut current_track = 0;
        let mut current_pulses = 0;
        for (track, section_len_change) in notes.section_len_changes() {
            while current_track < track.0 {
                pulses_at_track_start.insert(Track(current_track), current_pulses);
                current_pulses += resolution;
                current_track += 1;
            }

            debug_assert_eq!(current_track, track.0);
            pulses_at_track_start.insert(Track(current_track), current_pulses);
            current_pulses += (section_len_change.length * 4.0 * resolution as f64) as u32;
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
        PulseNumber(
            track_base
                + (self.resolution as f64 * time.numerator as f64 / time.denominator as f64) as u32,
        )
    }
}

#[test]
fn pulse_conversion() {
    use crate::{
        lex::command::{Key, NoteKind},
        parse::{notes::SectionLenChangeObj, obj::Obj},
    };

    // Source BMS:
    // ```
    // #00102:0.75
    // #00103:1.25
    // ```
    let notes = {
        let mut notes = Notes::default();
        notes.push_section_len_change(SectionLenChangeObj {
            track: Track(1),
            length: 0.75,
        });
        notes.push_section_len_change(SectionLenChangeObj {
            track: Track(2),
            length: 1.25,
        });
        notes.push_note(Obj {
            offset: ObjTime {
                track: Track(1),
                numerator: 2,
                denominator: 3,
            },
            kind: NoteKind::Visible,
            is_player1: true,
            key: Key::Key1,
            obj: 1.try_into().unwrap(),
        });
        notes
    };
    let converter = PulseConverter::new(&notes);

    assert_eq!(converter.resolution, 2);

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
        4
    );
    assert_eq!(
        converter
            .get_pulses_at(ObjTime {
                track: Track(1),
                numerator: 0,
                denominator: 4
            })
            .0,
        8
    );
    assert_eq!(
        converter
            .get_pulses_at(ObjTime {
                track: Track(1),
                numerator: 2,
                denominator: 4
            })
            .0,
        11
    );
    assert_eq!(
        converter
            .get_pulses_at(ObjTime {
                track: Track(2),
                numerator: 0,
                denominator: 4
            })
            .0,
        14
    );
    assert_eq!(
        converter
            .get_pulses_at(ObjTime {
                track: Track(2),
                numerator: 2,
                denominator: 4
            })
            .0,
        19
    );
    assert_eq!(
        converter
            .get_pulses_at(ObjTime {
                track: Track(3),
                numerator: 0,
                denominator: 4
            })
            .0,
        24
    );
}
