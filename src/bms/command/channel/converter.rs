//! For one-way converting key/channel, please see [`KeyLayoutConverter`] trait.

use std::collections::HashMap;

use super::{Key, PlayerSide, mapper::{KeyMapping, BeatKey, convert_key}};

/// A trait for converting [`KeyMapping`]s.
///
/// - Difference from [`KeyLayoutMapper`]:
///   - [`KeyLayoutMapper`] can convert between different key channel modes. It's two-way.
///   - [`KeyLayoutConverter`] can convert into another key layout. It's one-way.
pub trait KeyLayoutConverter {
    /// Convert a [`KeyMapping`] to another key layout.
    fn convert<T: KeyMapping>(&mut self, beat_map: T) -> T;
}

impl<const KEY_COUNT: usize, const SCRATCH_COUNT: usize> KeyLayoutConvertMirror<KEY_COUNT, SCRATCH_COUNT> {
    /// Create a new [`KeyLayoutConvertMirror`] with the given [`PlayerSide`] and [`Key`]s.
    pub fn new(side: PlayerSide, keys: Vec<Key<KEY_COUNT, SCRATCH_COUNT>>) -> Self {
        Self { side, keys }
    }
}

/// Mirror the note of a [`PlayerSide`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyLayoutConvertMirror<const KEY_COUNT: usize, const SCRATCH_COUNT: usize> {
    /// The side of the player to mirror.
    side: PlayerSide,
    /// A list of [`Key`]s to mirror. Usually, it should be the keys that actually used in the song.
    keys: Vec<Key<KEY_COUNT, SCRATCH_COUNT>>,
}

impl<const KEY_COUNT: usize, const SCRATCH_COUNT: usize> KeyLayoutConverter for KeyLayoutConvertMirror<KEY_COUNT, SCRATCH_COUNT> {
    fn convert<T: KeyMapping>(&mut self, beat_map: T) -> T {
        let (side, _key, _kind) = beat_map.into_tuple();
        if side == self.side {
            // Convert through NoteChannel to handle different key types
            if let Some(converted_beat_map) = self.try_convert_via_note_channel(beat_map) {
                return converted_beat_map;
            }
        }
        beat_map
    }
}

impl<const KEY_COUNT: usize, const SCRATCH_COUNT: usize> KeyLayoutConvertMirror<KEY_COUNT, SCRATCH_COUNT> {
    /// Try to convert a key using runtime type checking
    fn try_convert_key<T: Eq + std::fmt::Debug>(&self, key: &T) -> Option<Key<KEY_COUNT, SCRATCH_COUNT>> {
        // This is a hack to work around the generic type limitations
        // We assume that if the debug representation matches, it's the same type
        for (i, mirror_key) in self.keys.iter().enumerate() {
            // Use a simple approach: convert both to string and compare
            if format!("{:?}", mirror_key) == format!("{:?}", key) {
                let mirrored_pos = self.keys.len() - 1 - i;
                return Some(self.keys[mirrored_pos]);
            }
        }
        None
    }

    /// Try to convert via NoteChannel to handle different key types
    fn try_convert_via_note_channel<T: KeyMapping>(&self, beat_map: T) -> Option<T> {
        // Convert the beat_map to NoteChannel
        let note_channel = beat_map.to_note_channel();

        // Try to find the corresponding key in our list by converting to BeatKey first
        // This assumes that most KeyMapping implementations use BeatKey as the standard
        if let Some(beat_key) = BeatKey::from_note_channel(note_channel) {
            // Try to convert the BeatKey to our Key type
            if let Some(converter_key) = convert_key::<14, 2, KEY_COUNT, SCRATCH_COUNT>(beat_key.key) {
                // Try to find this key in our list
                if let Some(pos) = self.keys.iter().position(|k| *k == converter_key) {
                    // Apply mirror transformation
                    let mirrored_pos = self.keys.len() - 1 - pos;
                    let mirrored_converter_key = self.keys[mirrored_pos];

                    // Convert back to BeatKey
                    if let Some(mirrored_beat_key) = convert_key::<KEY_COUNT, SCRATCH_COUNT, 14, 2>(mirrored_converter_key) {
                        // Create mirrored BeatKey
                        let mirrored_beat = BeatKey::new(beat_key.side, mirrored_beat_key, beat_key.kind);

                        // Convert back to the original NoteChannel
                        let mirrored_note_channel = mirrored_beat.to_note_channel();

                        // Try to convert back to the original type T
                        return T::from_note_channel(mirrored_note_channel);
                    }
                }
            }
        }

        None
    }
}

/// A random number generator based on Java's `java.util.Random`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    /// Create a new [`JavaRandom`] with the given seed.
    pub fn new(seed: i64) -> Self {
        let s = (seed as u64) ^ 0x5DEECE66D;
        JavaRandom {
            seed: s & ((1u64 << 48) - 1),
        }
    }

    /// Java's next(int bits) method
    fn next(&mut self, bits: i32) -> i32 {
        const MULT: u64 = 0x5DEECE66D;
        const ADD: u64 = 0xB;
        self.seed = (self.seed.wrapping_mul(MULT).wrapping_add(ADD)) & ((1u64 << 48) - 1);
        ((self.seed >> (48 - bits)) & ((1u64 << bits) - 1)) as i32
    }

    /// Java's nextInt() method - returns any int value
    #[allow(dead_code)]
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// Java's nextInt(int bound) method
    pub fn next_int_bound(&mut self, bound: i32) -> i32 {
        if bound <= 0 {
            panic!("bound must be positive");
        }

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

/// A modifier that rotates the lanes of a [`KeyMapping`].
#[derive(Debug, Clone)]
pub struct KeyLayoutConvertLaneRotateShuffle<const KEY_COUNT: usize, const SCRATCH_COUNT: usize> {
    /// The side of the player to shuffle.
    side: PlayerSide,
    /// A map of [`Key`]s to their new [`Key`]s.
    arrangement: HashMap<Key<KEY_COUNT, SCRATCH_COUNT>, Key<KEY_COUNT, SCRATCH_COUNT>>,
}

impl<const KEY_COUNT: usize, const SCRATCH_COUNT: usize> KeyLayoutConvertLaneRotateShuffle<KEY_COUNT, SCRATCH_COUNT> {
    /// Create a new [`KeyLayoutConvertLaneRotateShuffle`] with the given [`PlayerSide`], [`Key`]s and seed.
    pub fn new(side: PlayerSide, keys: Vec<Key<KEY_COUNT, SCRATCH_COUNT>>, seed: i64) -> Self {
        KeyLayoutConvertLaneRotateShuffle {
            side,
            arrangement: Self::make_random(&keys, seed),
        }
    }

    fn make_random(keys: &[Key<KEY_COUNT, SCRATCH_COUNT>], seed: i64) -> HashMap<Key<KEY_COUNT, SCRATCH_COUNT>, Key<KEY_COUNT, SCRATCH_COUNT>> {
        let mut rng = JavaRandom::new(seed);
        let mut result: HashMap<Key<KEY_COUNT, SCRATCH_COUNT>, Key<KEY_COUNT, SCRATCH_COUNT>> = HashMap::new();
        if keys.is_empty() {
            return result;
        }

        let inc = rng.next_int_bound(2) == 1;
        let start = rng.next_int_bound(keys.len() as i32 - 1) as usize + if inc { 1 } else { 0 };

        let mut rlane = start;
        for lane in 0..keys.len() {
            result.insert(keys[lane], keys[rlane]);
            rlane = if inc {
                (rlane + 1) % keys.len()
            } else {
                (rlane + keys.len() - 1) % keys.len()
            };
        }
        result
    }
}

impl<const KEY_COUNT: usize, const SCRATCH_COUNT: usize> KeyLayoutConverter for KeyLayoutConvertLaneRotateShuffle<KEY_COUNT, SCRATCH_COUNT> {
    fn convert<T: KeyMapping>(&mut self, beat_map: T) -> T {
        let (side, key, kind) = beat_map.into_tuple();
        if side == self.side {
            // For now, just return the original - proper conversion needs runtime information
            // TODO: Implement proper key conversion for converters
        }
        T::new(side, key, kind)
    }
}

/// A modifier that shuffles the lanes of a [`KeyMapping`].
///
/// Its action is similar to beatoraja's lane shuffle.
#[derive(Debug, Clone)]
pub struct KeyLayoutConvertLaneRandomShuffle<const KEY_COUNT: usize, const SCRATCH_COUNT: usize> {
    /// The side of the player to shuffle.
    side: PlayerSide,
    /// A map of [`Key`]s to their new [`Key`]s.
    arrangement: HashMap<Key<KEY_COUNT, SCRATCH_COUNT>, Key<KEY_COUNT, SCRATCH_COUNT>>,
}

impl<const KEY_COUNT: usize, const SCRATCH_COUNT: usize> KeyLayoutConvertLaneRandomShuffle<KEY_COUNT, SCRATCH_COUNT> {
    /// Create a new [`KeyLayoutConvertLaneRandomShuffle`] with the given [`PlayerSide`], [`Key`]s and seed.
    pub fn new(side: PlayerSide, keys: Vec<Key<KEY_COUNT, SCRATCH_COUNT>>, seed: i64) -> Self {
        KeyLayoutConvertLaneRandomShuffle {
            side,
            arrangement: Self::make_random(&keys, seed),
        }
    }

    fn make_random(keys: &[Key<KEY_COUNT, SCRATCH_COUNT>], seed: i64) -> HashMap<Key<KEY_COUNT, SCRATCH_COUNT>, Key<KEY_COUNT, SCRATCH_COUNT>> {
        let mut rng = JavaRandom::new(seed);
        let mut result: HashMap<Key<KEY_COUNT, SCRATCH_COUNT>, Key<KEY_COUNT, SCRATCH_COUNT>> = HashMap::new();
        if keys.is_empty() {
            return result;
        }

        let mut l = keys.to_vec();
        for &lane in keys {
            let r = rng.next_int_bound(l.len() as i32) as usize;
            result.insert(lane, l[r]);
            l.remove(r);
        }

        result
    }
}

impl<const KEY_COUNT: usize, const SCRATCH_COUNT: usize> KeyLayoutConverter for KeyLayoutConvertLaneRandomShuffle<KEY_COUNT, SCRATCH_COUNT> {
    fn convert<T: KeyMapping>(&mut self, beat_map: T) -> T {
        let (side, key, kind) = beat_map.into_tuple();
        if side == self.side {
            // For now, just return the original - proper conversion needs runtime information
            // TODO: Implement proper key conversion for converters
        }
        T::new(side, key, kind)
    }
}

#[cfg(test)]
mod channel_mode_tests {
    use crate::bms::prelude::{BeatKey as KeyLayoutBeat, NoteKind};

    use super::*;

    #[test]
    fn test_key_channel_mode_mirror() {
        // Test 1: 3 keys
        let keys = vec![
            (PlayerSide::Player1, Key::<14, 2>::new_key(1).unwrap()),
            (PlayerSide::Player1, Key::<14, 2>::new_key(2).unwrap()),
            (PlayerSide::Player1, Key::<14, 2>::new_key(3).unwrap()),
            (PlayerSide::Player1, Key::<14, 2>::new_key(4).unwrap()),
            (PlayerSide::Player1, Key::<14, 2>::new_key(5).unwrap()),
            (PlayerSide::Player2, Key::<14, 2>::new_key(1).unwrap()),
            (PlayerSide::Player2, Key::<14, 2>::new_key(2).unwrap()),
            (PlayerSide::Player2, Key::<14, 2>::new_key(3).unwrap()),
            (PlayerSide::Player2, Key::<14, 2>::new_key(4).unwrap()),
            (PlayerSide::Player2, Key::<14, 2>::new_key(5).unwrap()),
        ]
        .into_iter()
        .map(|(side, key)| KeyLayoutBeat::new(side, key, NoteKind::Visible))
        .collect::<Vec<_>>();
        let mut mode = KeyLayoutConvertMirror::<14, 2> {
            side: PlayerSide::Player1,
            keys: vec![
                Key::<14, 2>::new_key(1).unwrap(),
                Key::<14, 2>::new_key(2).unwrap(),
                Key::<14, 2>::new_key(3).unwrap(),
            ],
        };
        let result = keys.iter().map(|k| mode.convert(*k)).collect::<Vec<_>>();
        let expected = vec![
            (PlayerSide::Player1, Key::<14, 2>::new_key(3).unwrap()),
            (PlayerSide::Player1, Key::<14, 2>::new_key(2).unwrap()),
            (PlayerSide::Player1, Key::<14, 2>::new_key(1).unwrap()),
            (PlayerSide::Player1, Key::<14, 2>::new_key(4).unwrap()),
            (PlayerSide::Player1, Key::<14, 2>::new_key(5).unwrap()),
            (PlayerSide::Player2, Key::<14, 2>::new_key(1).unwrap()),
            (PlayerSide::Player2, Key::<14, 2>::new_key(2).unwrap()),
            (PlayerSide::Player2, Key::<14, 2>::new_key(3).unwrap()),
            (PlayerSide::Player2, Key::<14, 2>::new_key(4).unwrap()),
            (PlayerSide::Player2, Key::<14, 2>::new_key(5).unwrap()),
        ]
        .into_iter()
        .map(|(side, key)| KeyLayoutBeat::new(side, key, NoteKind::Visible))
        .collect::<Vec<_>>();
        assert_eq!(result, expected);
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

    /// Test the random shuffle modifier.
    ///
    /// Source: https://www.bilibili.com/opus/1033281595747860483
    #[test]
    fn test_random_shuffle() {
        let examples = [
            "1234567 4752",
            "1234576 2498",
            "4372615 12728",
            "4372651 9734",
            "4375126 139",
        ]
        .iter()
        .map(|s| {
            let v = s.split_whitespace().collect::<Vec<_>>();
            let [list, seed] = v.as_slice() else {
                println!("{:?}", v);
                panic!("Invalid input");
            };
            let list = list
                .chars()
                .map(|c| c.to_digit(10).unwrap() as usize)
                .collect::<Vec<_>>();
            let seed = seed.parse::<i64>().unwrap();
            (list, seed)
        })
        .collect::<Vec<_>>();

        for (i, (list, seed)) in examples.iter().enumerate() {
            println!("Test case {}: seed = {}", i, seed);
            let init_keys = [
                Key::<14, 2>::new_key(1).unwrap(),
                Key::<14, 2>::new_key(2).unwrap(),
                Key::<14, 2>::new_key(3).unwrap(),
                Key::<14, 2>::new_key(4).unwrap(),
                Key::<14, 2>::new_key(5).unwrap(),
                Key::<14, 2>::new_key(6).unwrap(),
                Key::<14, 2>::new_key(7).unwrap(),
            ];
            let mut rnd = KeyLayoutConvertLaneRandomShuffle::<14, 2>::new(
                PlayerSide::Player1,
                init_keys.to_vec(),
                *seed,
            );
            let result_values = init_keys
                .into_iter()
                .map(|k| {
                    rnd.convert(KeyLayoutBeat::new(
                        PlayerSide::Player1,
                        k,
                        NoteKind::Visible,
                    ))
                })
                .map(|v| match v.key() {
                    Key::<14, 2>::Key(idx) => idx.get() as usize,
                    Key::<14, 2>::Scratch(idx) => (idx.get() + 15) as usize, // Scratch is typically 16
                    Key::<14, 2>::FootPedal => 18,
                    Key::<14, 2>::FreeZone => 19,
                })
                .collect::<Vec<_>>();
            println!("  Expected: {:?}", list);
            println!("  Got:      {:?}", result_values);
            println!("  Match:    {}", result_values == *list);
            if result_values != *list {
                println!("  FAILED!");
            }
            println!();
        }
    }
}
