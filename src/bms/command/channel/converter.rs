//! For one-way converting key/channel, please see [`KeyConverter`] and [`PlayerSideKeyConverter`] traits.

use std::collections::HashMap;

use super::{Key, PlayerSide};
use crate::bms::ast::rng::JavaRandom;

/// A trait for converting [`Key`]s in different layouts.
///
/// This trait provides a simple interface for converting keys without considering player sides.
/// It operates on iterators of keys, making it suitable for key-only transformations.
pub trait KeyConverter {
    /// Convert an iterator of [`Key`]s to another key layout.
    fn convert(&mut self, keys: impl Iterator<Item = Key>) -> impl Iterator<Item = Key>;
}

/// A trait for converting [`PlayerSide`] and [`Key`] pairs in different layouts.
///
/// This trait provides an interface for converting (PlayerSide, Key) pairs,
/// making it suitable for transformations that need to consider both player side and key.
pub trait PlayerSideKeyConverter {
    /// Convert an iterator of `(PlayerSide, Key)` pairs to another layout.
    fn convert(
        &mut self,
        pairs: impl Iterator<Item = (PlayerSide, Key)>,
    ) -> impl Iterator<Item = (PlayerSide, Key)>;
}

impl KeyMappingConvertMirror {
    /// Create a new [`KeyMappingConvertMirror`] with the given [`Key`]s.
    #[must_use]
    pub const fn new(keys: Vec<Key>) -> Self {
        Self { keys }
    }
}

/// Mirror the keys within the specified key list.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyMappingConvertMirror {
    /// A list of [`Key`]s to mirror. Usually, it should be the keys that actually used in the song.
    keys: Vec<Key>,
}

impl KeyConverter for KeyMappingConvertMirror {
    fn convert(&mut self, keys: impl Iterator<Item = Key>) -> impl Iterator<Item = Key> {
        keys.map(|key| {
            self.keys
                .iter()
                .position(|k| k == &key)
                .and_then(|position| {
                    let mirror_index = self.keys.len().saturating_sub(position + 1);
                    self.keys.get(mirror_index)
                })
                .copied()
                .unwrap_or(key)
        })
    }
}

/// A modifier that rotates the lanes of keys.
#[derive(Debug, Clone)]
pub struct KeyMappingConvertLaneRotateShuffle {
    /// A map of [`Key`]s to their new [`Key`]s.
    arrangement: HashMap<Key, Key>,
}

impl KeyMappingConvertLaneRotateShuffle {
    /// Create a new [`KeyMappingConvertLaneRotateShuffle`] with the given [`Key`]s and seed.
    #[must_use]
    pub fn new(keys: &[Key], seed: i64) -> Self {
        Self {
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

impl KeyConverter for KeyMappingConvertLaneRotateShuffle {
    fn convert(&mut self, keys: impl Iterator<Item = Key>) -> impl Iterator<Item = Key> {
        keys.map(|key| self.arrangement.get(&key).copied().unwrap_or(key))
    }
}

/// A modifier that shuffles the lanes of keys.
///
/// Its action is similar to beatoraja's lane shuffle.
#[derive(Debug, Clone)]
pub struct KeyMappingConvertLaneRandomShuffle {
    /// A map of [`Key`]s to their new [`Key`]s.
    arrangement: HashMap<Key, Key>,
}

impl KeyMappingConvertLaneRandomShuffle {
    /// Create a new [`KeyMappingConvertLaneRandomShuffle`] with the given [`Key`]s and seed.
    #[must_use]
    pub fn new(keys: &[Key], seed: i64) -> Self {
        Self {
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

impl KeyConverter for KeyMappingConvertLaneRandomShuffle {
    fn convert(&mut self, keys: impl Iterator<Item = Key>) -> impl Iterator<Item = Key> {
        keys.map(|key| self.arrangement.get(&key).copied().unwrap_or(key))
    }
}

/// A modifier that flips between PlayerSide::Player1 and PlayerSide::Player2.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyMappingConvertFlip;

impl PlayerSideKeyConverter for KeyMappingConvertFlip {
    fn convert(
        &mut self,
        pairs: impl Iterator<Item = (PlayerSide, Key)>,
    ) -> impl Iterator<Item = (PlayerSide, Key)> {
        pairs.map(|(side, key)| {
            let flipped_side = match side {
                PlayerSide::Player1 => PlayerSide::Player2,
                PlayerSide::Player2 => PlayerSide::Player1,
            };
            (flipped_side, key)
        })
    }
}

#[cfg(test)]
mod channel_mode_tests {
    use super::*;

    #[test]
    fn test_key_converter_mirror() {
        // Test 1: 3 keys
        let mut converter =
            KeyMappingConvertMirror::new(vec![Key::Key(1), Key::Key(2), Key::Key(3)]);

        // Test individual key conversions
        let keys = vec![
            Key::Key(1),
            Key::Key(2),
            Key::Key(3),
            Key::Key(4),
            Key::Key(5),
        ];

        // Expected keys (mirrored: 1->3, 2->2, 3->1, 4->4, 5->5)
        let expected_keys = vec![
            Key::Key(3),
            Key::Key(2),
            Key::Key(1),
            Key::Key(4),
            Key::Key(5),
        ];

        let result: Vec<_> = converter.convert(keys.into_iter()).collect();
        assert_eq!(result, expected_keys);
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
        T: KeyConverter,
    {
        println!("Test case {}: seed = {}", test_case_idx, seed);

        let result_values = converter
            .convert(keys.iter().copied())
            .map(|key| key_to_value(key))
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
            let rnd = KeyMappingConvertLaneRandomShuffle::new(&init_keys, *seed);
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
            let rnd = KeyMappingConvertLaneRotateShuffle::new(&init_keys, *seed);
            run_shuffle_test_case(i, list, *seed, &init_keys, rnd);
        }
    }

    /// Test the flip modifier that swaps PlayerSide::Player1 and PlayerSide::Player2.
    #[test]
    fn test_player_side_key_converter_flip() {
        let mut converter = KeyMappingConvertFlip::default();

        // Test data: (PlayerSide, Key)
        let test_cases = vec![
            (PlayerSide::Player1, Key::Key(1)),
            (PlayerSide::Player2, Key::Key(2)),
            (PlayerSide::Player1, Key::Scratch(1)),
            (PlayerSide::Player2, Key::FreeZone),
        ];

        // Test flip conversion
        let input_pairs: Vec<_> = test_cases.clone();

        let expected_pairs: Vec<_> = test_cases
            .iter()
            .map(|(side, key)| {
                let flipped_side = match side {
                    PlayerSide::Player1 => PlayerSide::Player2,
                    PlayerSide::Player2 => PlayerSide::Player1,
                };
                (flipped_side, *key)
            })
            .collect();

        let result: Vec<_> = converter.convert(input_pairs.clone().into_iter()).collect();
        assert_eq!(&result, &expected_pairs);

        let result2: Vec<_> = converter.convert(result.into_iter()).collect();
        assert_eq!(&result2, &input_pairs);
    }
}
