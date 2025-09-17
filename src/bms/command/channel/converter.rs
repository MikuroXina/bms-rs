//! For one-way converting key/channel, please see [`KeyMappingConverter`] trait.

use std::collections::HashMap;

use super::{Key, PlayerSide, mapper::KeyMapping};
use crate::bms::ast::rng::JavaRandom;
use crate::bms::command::time::ObjTime;

/// A trait for converting [`KeyMapping`]s in different layouts.
///
/// - Difference from [`super::mapper::KeyLayoutMapper`]:
///   - [`super::mapper::KeyLayoutMapper`] can convert between different key channel modes. It's two-way.
///   - [`KeyMappingConverter`] can convert into another key layout. It's one-way.
///   - [`KeyMappingConverter`] operates on iterators of `(ObjTime, KeyMapping)` pairs, preserving timing information.
pub trait KeyMappingConverter {
    /// Convert an iterator of `(ObjTime, KeyMapping)` pairs to another key layout.
    fn convert<T: KeyMapping>(
        &mut self,
        mappings: impl Iterator<Item = (ObjTime, T)>,
    ) -> impl Iterator<Item = (ObjTime, T)>;
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

impl KeyMappingConverter for KeyMappingConvertMirror {
    fn convert<T: KeyMapping>(
        &mut self,
        mappings: impl Iterator<Item = (ObjTime, T)>,
    ) -> impl Iterator<Item = (ObjTime, T)> {
        mappings.map(|(time, mapping)| {
            let (side, kind, key) = mapping.as_tuple();
            let converted_key = self
                .keys
                .iter()
                .position(|k| k == &key)
                .and_then(|position| {
                    let mirror_index = self.keys.len().saturating_sub(position + 1);
                    self.keys.get(mirror_index)
                })
                .copied()
                .unwrap_or(key);
            (time, T::new(side, kind, converted_key))
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

impl KeyMappingConverter for KeyMappingConvertLaneRotateShuffle {
    fn convert<T: KeyMapping>(
        &mut self,
        mappings: impl Iterator<Item = (ObjTime, T)>,
    ) -> impl Iterator<Item = (ObjTime, T)> {
        mappings.map(|(time, mapping)| {
            let (side, kind, key) = mapping.as_tuple();
            let converted_key = self.arrangement.get(&key).copied().unwrap_or(key);
            (time, T::new(side, kind, converted_key))
        })
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

impl KeyMappingConverter for KeyMappingConvertLaneRandomShuffle {
    fn convert<T: KeyMapping>(
        &mut self,
        mappings: impl Iterator<Item = (ObjTime, T)>,
    ) -> impl Iterator<Item = (ObjTime, T)> {
        mappings.map(|(time, mapping)| {
            let (side, kind, key) = mapping.as_tuple();
            let converted_key = self.arrangement.get(&key).copied().unwrap_or(key);
            (time, T::new(side, kind, converted_key))
        })
    }
}

/// A modifier that flips between PlayerSide::Player1 and PlayerSide::Player2.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyMappingConvertFlip;

impl KeyMappingConverter for KeyMappingConvertFlip {
    fn convert<T: KeyMapping>(
        &mut self,
        mappings: impl Iterator<Item = (ObjTime, T)>,
    ) -> impl Iterator<Item = (ObjTime, T)> {
        mappings.map(|(time, mapping)| {
            let (side, kind, key) = mapping.as_tuple();
            let flipped_side = match side {
                PlayerSide::Player1 => PlayerSide::Player2,
                PlayerSide::Player2 => PlayerSide::Player1,
            };
            (time, T::new(flipped_side, kind, key))
        })
    }
}

#[cfg(test)]
mod channel_mode_tests {
    use super::*;
    use crate::bms::command::time::ObjTime;
    use crate::bms::prelude::*;

    /// Create a (ObjTime, KeyLayoutBeat) pair for testing.
    fn create_mapping(
        track: u64,
        position: u64,
        denominator: u64,
        side: PlayerSide,
        kind: NoteKind,
        key: Key,
    ) -> (ObjTime, KeyLayoutBeat) {
        (
            ObjTime::new(track, position, denominator),
            KeyLayoutBeat::new(side, kind, key),
        )
    }

    /// Create a sequence of mappings with the same parameters but different keys.
    fn create_mappings(
        keys: impl IntoIterator<Item = Key>,
        track: u64,
        denominator: u64,
        side: PlayerSide,
        kind: NoteKind,
    ) -> impl Iterator<Item = (ObjTime, KeyLayoutBeat)> {
        keys.into_iter()
            .enumerate()
            .map(move |(i, key)| create_mapping(track, i as u64, denominator, side, kind, key))
    }

    #[test]
    fn test_key_channel_mode_mirror() {
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
        let input_mappings: Vec<_> =
            create_mappings(keys.clone(), 0, 4, PlayerSide::Player1, NoteKind::Visible).collect();

        // Expected mappings (mirrored: 1->3, 2->2, 3->1, 4->4, 5->5)
        let expected_keys = vec![
            Key::Key(3),
            Key::Key(2),
            Key::Key(1),
            Key::Key(4),
            Key::Key(5),
        ];
        let expected_mappings: Vec<_> =
            create_mappings(expected_keys, 0, 4, PlayerSide::Player1, NoteKind::Visible).collect();

        let result: Vec<_> = converter.convert(input_mappings.into_iter()).collect();
        assert_eq!(result, expected_mappings);
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
        T: KeyMappingConverter,
    {
        println!("Test case {}: seed = {}", test_case_idx, seed);

        let mappings: Vec<_> = create_mappings(
            keys.iter().copied(),
            0,
            keys.len() as u64,
            PlayerSide::Player1,
            NoteKind::Visible,
        )
        .collect();

        let result_values = converter
            .convert(mappings.into_iter())
            .map(|(_, mapping)| key_to_value(mapping.key()))
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
    fn test_key_mapping_convert_flip() {
        let mut converter = KeyMappingConvertFlip::default();

        // Test data: (PlayerSide, NoteKind, Key)
        let test_cases = vec![
            (PlayerSide::Player1, NoteKind::Visible, Key::Key(1)),
            (PlayerSide::Player2, NoteKind::Long, Key::Key(2)),
            (PlayerSide::Player1, NoteKind::Invisible, Key::Scratch(1)),
            (PlayerSide::Player2, NoteKind::Landmine, Key::FreeZone),
        ];

        // Test flip conversion
        let input_mappings: Vec<_> = test_cases
            .iter()
            .enumerate()
            .map(|(i, (side, kind, key))| create_mapping(0, i as u64, 4, *side, *kind, *key))
            .collect();

        let expected_mappings: Vec<_> = test_cases
            .iter()
            .enumerate()
            .map(|(i, (side, kind, key))| {
                let flipped_side = match side {
                    PlayerSide::Player1 => PlayerSide::Player2,
                    PlayerSide::Player2 => PlayerSide::Player1,
                };
                create_mapping(0, i as u64, 4, flipped_side, *kind, *key)
            })
            .collect();

        let result: Vec<_> = converter
            .convert(input_mappings.clone().into_iter())
            .collect();
        assert_eq!(&result, &expected_mappings);

        let result2: Vec<_> = converter.convert(result.into_iter()).collect();
        assert_eq!(&result2, &input_mappings);
    }
}
