//! For one-way converting key/channel, please see [`KeyLayoutConverter`] trait.

use std::collections::HashMap;

use super::{Key, PlayerSide, mapper::KeyMapping};
use crate::bms::ast::rng::JavaRandom;

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
    #[must_use]
    pub const fn new(side: PlayerSide, keys: Vec<Key>) -> Self {
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
        let (side, kind, mut key) = beat_map.into_tuple();
        if side == self.side
            && let Some(position) = self.keys.iter().position(|k| k == &key)
        {
            let mirror_index = self.keys.len().saturating_sub(position + 1);
            let Some(mirror_key) = self.keys.get(mirror_index) else {
                return T::new(side, kind, key);
            };
            key = *mirror_key;
        }
        T::new(side, kind, key)
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
    #[must_use]
    pub fn new(side: PlayerSide, keys: &[Key], seed: i64) -> Self {
        Self {
            side,
            arrangement: Self::make_random(keys, seed),
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
        let (side, kind, key) = beat_map.into_tuple();
        if side == self.side {
            let new_key = self.arrangement.get(&key).copied().unwrap_or(key);
            T::new(side, kind, new_key)
        } else {
            T::new(side, kind, key)
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
    #[must_use]
    pub fn new(side: PlayerSide, keys: &[Key], seed: i64) -> Self {
        Self {
            side,
            arrangement: Self::make_random(keys, seed),
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
        let (side, kind, key) = beat_map.into_tuple();
        if side == self.side {
            let new_key = self.arrangement.get(&key).copied().unwrap_or(key);
            T::new(side, kind, new_key)
        } else {
            T::new(side, kind, key)
        }
    }
}

#[cfg(test)]
mod channel_mode_tests {
    use crate::bms::prelude::{KeyLayoutBeat, NoteKind};

    use super::*;

    #[test]
    fn test_key_channel_mode_mirror() {
        // Test 1: 3 keys
        let keys = vec![
            (PlayerSide::Player1, Key::Key(1)),
            (PlayerSide::Player1, Key::Key(2)),
            (PlayerSide::Player1, Key::Key(3)),
            (PlayerSide::Player1, Key::Key(4)),
            (PlayerSide::Player1, Key::Key(5)),
            (PlayerSide::Player2, Key::Key(1)),
            (PlayerSide::Player2, Key::Key(2)),
            (PlayerSide::Player2, Key::Key(3)),
            (PlayerSide::Player2, Key::Key(4)),
            (PlayerSide::Player2, Key::Key(5)),
        ]
        .into_iter()
        .map(|(side, key)| KeyLayoutBeat::new(side, NoteKind::Visible, key))
        .collect::<Vec<_>>();
        let mut mode = KeyLayoutConvertMirror {
            side: PlayerSide::Player1,
            keys: vec![Key::Key(1), Key::Key(2), Key::Key(3)],
        };
        let result = keys.iter().map(|k| mode.convert(*k)).collect::<Vec<_>>();
        let expected = vec![
            (PlayerSide::Player1, Key::Key(3)),
            (PlayerSide::Player1, Key::Key(2)),
            (PlayerSide::Player1, Key::Key(1)),
            (PlayerSide::Player1, Key::Key(4)),
            (PlayerSide::Player1, Key::Key(5)),
            (PlayerSide::Player2, Key::Key(1)),
            (PlayerSide::Player2, Key::Key(2)),
            (PlayerSide::Player2, Key::Key(3)),
            (PlayerSide::Player2, Key::Key(4)),
            (PlayerSide::Player2, Key::Key(5)),
        ]
        .into_iter()
        .map(|(side, key)| KeyLayoutBeat::new(side, NoteKind::Visible, key))
        .collect::<Vec<_>>();
        assert_eq!(result, expected);
    }

    /// Parse test examples from string format to (list, seed) tuples.
    fn parse_examples(examples_str: &[&str]) -> Vec<(Vec<usize>, i64)> {
        examples_str
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
            .collect::<Vec<_>>()
    }

    /// Convert a Key to its numeric value for testing.
    fn key_to_value(key: Key) -> usize {
        match key {
            Key::Key(n) => n as usize,
            Key::Scratch(n) => n as usize + 10,
            Key::FootPedal => 20,
            Key::FreeZone => 21,
        }
    }

    /// Run a single shuffle test case with given keys.
    fn run_shuffle_test_case<T>(
        test_case_idx: usize,
        expected_list: &[usize],
        seed: i64,
        keys: &[Key],
        mut converter: T,
    ) where
        T: KeyLayoutConverter,
    {
        println!("Test case {}: seed = {}", test_case_idx, seed);

        let result_values = keys
            .iter()
            .map(|&k| {
                converter.convert(KeyLayoutBeat::new(
                    PlayerSide::Player1,
                    NoteKind::Visible,
                    k,
                ))
            })
            .map(|v| key_to_value(v.key()))
            .collect::<Vec<_>>();

        println!("  Expected: {:?}", expected_list);
        println!("  Got:      {:?}", result_values);
        println!("  Match:    {}", result_values == expected_list);

        if result_values != expected_list {
            println!("  FAILED!");
        }
        println!();
    }

    /// Test the random shuffle modifier.
    ///
    /// Source: <https://www.bilibili.com/opus/1033281595747860483>
    #[test]
    fn test_random_shuffle() {
        let examples_str = [
            "1234567 4752",
            "1234576 2498",
            "4372615 12728",
            "4372651 9734",
            "4375126 139",
        ];
        let examples = parse_examples(&examples_str);
        let init_keys = [
            Key::Key(1),
            Key::Key(2),
            Key::Key(3),
            Key::Key(4),
            Key::Key(5),
            Key::Key(6),
            Key::Key(7),
        ];

        for (i, (list, seed)) in examples.iter().enumerate() {
            let rnd =
                KeyLayoutConvertLaneRandomShuffle::new(PlayerSide::Player1, &init_keys, *seed);
            run_shuffle_test_case(i, list, *seed, &init_keys, rnd);
        }
    }

    /// Test the lane rotate shuffle modifier.
    #[test]
    fn test_lane_rotate_shuffle() {
        let examples_str = ["1765432 3581225"];
        let examples = parse_examples(&examples_str);
        let init_keys = [
            Key::Key(1),
            Key::Key(2),
            Key::Key(3),
            Key::Key(4),
            Key::Key(5),
            Key::Key(6),
            Key::Key(7),
        ];

        for (i, (list, seed)) in examples.iter().enumerate() {
            let rnd =
                KeyLayoutConvertLaneRotateShuffle::new(PlayerSide::Player1, &init_keys, *seed);
            run_shuffle_test_case(i, list, *seed, &init_keys, rnd);
        }
    }
}
