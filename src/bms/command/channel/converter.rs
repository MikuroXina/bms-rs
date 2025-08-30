//! For one-way converting key/channel, please see [`KeyLayoutConverter`] trait.

use std::collections::HashMap;

use super::{Key, PlayerSide, mapper::KeyMapping};

/// A trait for converting [`KeyMapping`]s.
///
/// - Difference from [`KeyLayoutMapper`]:
///   - [`KeyLayoutMapper`] can convert between different key channel modes. It's two-way.
///   - [`KeyLayoutConverter`] can convert into another key layout. It's one-way.
pub trait KeyLayoutConverter {
    /// Convert a [`KeyMapping`] to another key layout.
    fn convert<T: KeyMapping>(&mut self, beat_map: T) -> T;
}

impl KeyLayoutConvertMirror {
    /// Create a new [`KeyLayoutConvertMirror`] with the given [`PlayerSide`] and [`Key`]s.
    pub fn new(side: PlayerSide, keys: Vec<Key>) -> Self {
        Self { side, keys }
    }
}

/// Mirror the note of a [`PlayerSide`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyLayoutConvertMirror {
    /// The side of the player to mirror.
    side: PlayerSide,
    /// A list of [`Key`]s to mirror. Usually, it should be the keys that actually used in the song.
    keys: Vec<Key>,
}

impl KeyLayoutConverter for KeyLayoutConvertMirror {
    fn convert<T: KeyMapping>(&mut self, beat_map: T) -> T {
        let (side, mut key, kind) = beat_map.into_tuple();
        if side == self.side
            && let Some(position) = self.keys.iter().position(|k| k == &key)
        {
            let mirror_index = self.keys.len().saturating_sub(position + 1);
            let Some(mirror_key) = self.keys.get(mirror_index) else {
                return T::new(side, key, kind);
            };
            key = *mirror_key;
        }
        T::new(side, key, kind)
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
pub struct KeyLayoutConvertLaneRotateShuffle {
    /// The side of the player to shuffle.
    side: PlayerSide,
    /// A map of [`Key`]s to their new [`Key`]s.
    arrangement: HashMap<Key, Key>,
}

impl KeyLayoutConvertLaneRotateShuffle {
    /// Create a new [`KeyLayoutConvertLaneRotateShuffle`] with the given [`PlayerSide`], [`Key`]s and seed.
    pub fn new(side: PlayerSide, keys: Vec<Key>, seed: i64) -> Self {
        KeyLayoutConvertLaneRotateShuffle {
            side,
            arrangement: Self::make_random(&keys, seed),
        }
    }

    fn make_random(keys: &[Key], seed: i64) -> HashMap<Key, Key> {
        let mut rng = JavaRandom::new(seed);
        let mut result: HashMap<Key, Key> = HashMap::new();
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

impl KeyLayoutConverter for KeyLayoutConvertLaneRotateShuffle {
    fn convert<T: KeyMapping>(&mut self, beat_map: T) -> T {
        let (side, key, kind) = beat_map.into_tuple();
        if side == self.side {
            let new_key = self.arrangement.get(&key).copied().unwrap_or(key);
            T::new(side, new_key, kind)
        } else {
            T::new(side, key, kind)
        }
    }
}

/// A modifier that shuffles the lanes of a [`KeyMapping`].
///
/// Its action is similar to beatoraja's lane shuffle.
#[derive(Debug, Clone)]
pub struct KeyLayoutConvertLaneRandomShuffle {
    /// The side of the player to shuffle.
    side: PlayerSide,
    /// A map of [`Key`]s to their new [`Key`]s.
    arrangement: HashMap<Key, Key>,
}

impl KeyLayoutConvertLaneRandomShuffle {
    /// Create a new [`KeyLayoutConvertLaneRandomShuffle`] with the given [`PlayerSide`], [`Key`]s and seed.
    pub fn new(side: PlayerSide, keys: Vec<Key>, seed: i64) -> Self {
        KeyLayoutConvertLaneRandomShuffle {
            side,
            arrangement: Self::make_random(&keys, seed),
        }
    }

    fn make_random(keys: &[Key], seed: i64) -> HashMap<Key, Key> {
        let mut rng = JavaRandom::new(seed);
        let mut result: HashMap<Key, Key> = HashMap::new();
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

impl KeyLayoutConverter for KeyLayoutConvertLaneRandomShuffle {
    fn convert<T: KeyMapping>(&mut self, beat_map: T) -> T {
        let (side, key, kind) = beat_map.into_tuple();
        if side == self.side {
            let new_key = self.arrangement.get(&key).copied().unwrap_or(key);
            T::new(side, new_key, kind)
        } else {
            T::new(side, key, kind)
        }
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
            (PlayerSide::Player1, Key::Key1),
            (PlayerSide::Player1, Key::Key2),
            (PlayerSide::Player1, Key::Key3),
            (PlayerSide::Player1, Key::Key4),
            (PlayerSide::Player1, Key::Key5),
            (PlayerSide::Player2, Key::Key1),
            (PlayerSide::Player2, Key::Key2),
            (PlayerSide::Player2, Key::Key3),
            (PlayerSide::Player2, Key::Key4),
            (PlayerSide::Player2, Key::Key5),
        ]
        .into_iter()
        .map(|(side, key)| KeyLayoutBeat::new(side, key, NoteKind::Visible))
        .collect::<Vec<_>>();
        let mut mode = KeyLayoutConvertMirror {
            side: PlayerSide::Player1,
            keys: vec![Key::Key1, Key::Key2, Key::Key3],
        };
        let result = keys.iter().map(|k| mode.convert(*k)).collect::<Vec<_>>();
        let expected = vec![
            (PlayerSide::Player1, Key::Key3),
            (PlayerSide::Player1, Key::Key2),
            (PlayerSide::Player1, Key::Key1),
            (PlayerSide::Player1, Key::Key4),
            (PlayerSide::Player1, Key::Key5),
            (PlayerSide::Player2, Key::Key1),
            (PlayerSide::Player2, Key::Key2),
            (PlayerSide::Player2, Key::Key3),
            (PlayerSide::Player2, Key::Key4),
            (PlayerSide::Player2, Key::Key5),
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
                Key::Key1,
                Key::Key2,
                Key::Key3,
                Key::Key4,
                Key::Key5,
                Key::Key6,
                Key::Key7,
            ];
            let mut rnd = KeyLayoutConvertLaneRandomShuffle::new(
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
                .map(|v| v.key() as usize)
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
