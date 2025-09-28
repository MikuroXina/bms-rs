//! Definitions of command argument data.
//!
//! Structures in this module can be used in [Lex] part, [Parse] part, and the output models.

use std::collections::{HashMap, HashSet, VecDeque};

pub mod channel;
pub mod graphics;
pub mod mixin;
pub mod time;

/// Minor command types and utilities.
///
/// This module contains types and utilities for minor BMS commands that are only available
/// when the `minor-command` feature is enabled.
pub mod minor_command;

/// A play style of the score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlayerMode {
    /// For single play, a player uses 5 or 7 keys.
    Single,
    /// For couple play, two players use each 5 or 7 keys.
    Two,
    /// For double play, a player uses 10 or 14 keys.
    Double,
}

/// A rank to determine judge level, but treatment differs among the BMS players.
///
/// IIDX/LR2/beatoraja judge windows: <https://iidx.org/misc/iidx_lr2_beatoraja_diff>
///
/// Note: The difficulty `VeryEasy` is decided to be unimplemented.
/// See [discussions in the PR](https://github.com/MikuroXina/bms-rs/pull/122).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum JudgeLevel {
    /// Rank 0, the most difficult rank.
    VeryHard,
    /// Rank 1, the harder rank.
    Hard,
    /// Rank 2, the normal rank.
    Normal,
    /// Rank 3, the easier rank.
    Easy,
    /// Other integer value. Please See `JudgeLevel` for more details.
    /// If used for `#EXRANK`, representing percentage.
    OtherInt(i64),
}

impl From<i64> for JudgeLevel {
    fn from(value: i64) -> Self {
        match value {
            0 => Self::VeryHard,
            1 => Self::Hard,
            2 => Self::Normal,
            3 => Self::Easy,
            val => Self::OtherInt(val),
        }
    }
}

impl<'a> TryFrom<&'a str> for JudgeLevel {
    type Error = &'a str;
    fn try_from(value: &'a str) -> core::result::Result<Self, Self::Error> {
        value.parse::<i64>().map(Self::from).map_err(|_| value)
    }
}

impl std::fmt::Display for JudgeLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VeryHard => write!(f, "0"),
            Self::Hard => write!(f, "1"),
            Self::Normal => write!(f, "2"),
            Self::Easy => write!(f, "3"),
            Self::OtherInt(value) => write!(f, "{value}"),
        }
    }
}

pub(crate) const fn char_to_base62(ch: char) -> Option<u8> {
    match ch {
        '0'..='9' | 'A'..='Z' | 'a'..='z' => Some(ch as u32 as u8),
        _ => None,
    }
}

pub(crate) fn base62_to_byte(base62: u8) -> u8 {
    match base62 {
        b'0'..=b'9' => base62 - b'0',
        b'A'..=b'Z' => base62 - b'A' + 10,
        b'a'..=b'z' => base62 - b'a' + 36,
        _ => unreachable!(),
    }
}

/// An object id. Its meaning is determined by the channel belonged to.
///
/// The representation is 2 digits of ASCII characters.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjId([u8; 2]);

impl std::fmt::Debug for ObjId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObjId")
            .field(&format!("{}{}", self.0[0] as char, self.0[1] as char))
            .finish()
    }
}

impl std::fmt::Display for ObjId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.0[0] as char, self.0[1] as char)
    }
}

impl TryFrom<[char; 2]> for ObjId {
    type Error = [char; 2];
    fn try_from(value: [char; 2]) -> core::result::Result<Self, Self::Error> {
        Ok(Self([
            char_to_base62(value[0]).ok_or(value)?,
            char_to_base62(value[1]).ok_or(value)?,
        ]))
    }
}

impl TryFrom<[u8; 2]> for ObjId {
    type Error = [u8; 2];

    fn try_from(value: [u8; 2]) -> core::result::Result<Self, Self::Error> {
        <Self as TryFrom<[char; 2]>>::try_from([value[0] as char, value[1] as char])
            .map_err(|_| value)
    }
}

impl<'a> TryFrom<&'a str> for ObjId {
    type Error = &'a str;
    fn try_from(value: &'a str) -> core::result::Result<Self, Self::Error> {
        if value.len() != 2 {
            return Err(value);
        }
        let mut chars = value.bytes();
        let [Some(ch1), Some(ch2), None] = [chars.next(), chars.next(), chars.next()] else {
            return Err(value);
        };
        Self::try_from([ch1, ch2]).map_err(|_| value)
    }
}

impl From<ObjId> for u16 {
    fn from(value: ObjId) -> Self {
        base62_to_byte(value.0[0]) as u16 * 62 + base62_to_byte(value.0[1]) as u16
    }
}

impl From<ObjId> for u32 {
    fn from(value: ObjId) -> Self {
        Into::<u16>::into(value) as u32
    }
}

impl From<ObjId> for u64 {
    fn from(value: ObjId) -> Self {
        Into::<u16>::into(value) as u64
    }
}

impl ObjId {
    /// Instances a special null id, which means the rest object.
    #[must_use]
    pub const fn null() -> Self {
        Self([0, 0])
    }

    /// Returns whether the id is `00`.
    #[must_use]
    pub fn is_null(self) -> bool {
        self.0 == [0, 0]
    }

    /// Converts the object id into an `u16` value.
    #[must_use]
    pub fn as_u16(self) -> u16 {
        self.into()
    }

    /// Converts the object id into an `u32` value.
    #[must_use]
    pub fn as_u32(self) -> u32 {
        self.into()
    }

    /// Converts the object id into an `u64` value.
    #[must_use]
    pub fn as_u64(self) -> u64 {
        self.into()
    }

    /// Makes the object id uppercase.
    pub const fn make_uppercase(&mut self) {
        self.0[0] = self.0[0].to_ascii_uppercase();
        self.0[1] = self.0[1].to_ascii_uppercase();
    }

    /// Returns whether both characters are valid Base36 characters (0-9, A-Z).
    #[must_use]
    pub fn is_base36(self) -> bool {
        self.0
            .iter()
            .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
    }

    /// Returns whether both characters are valid Base62 characters (0-9, A-Z, a-z).
    #[must_use]
    pub fn is_base62(self) -> bool {
        self.0
            .iter()
            .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase() || c.is_ascii_lowercase())
    }

    /// Returns an iterator over all possible ObjId values, ordered by priority:
    /// first all Base36 values (0-9, A-Z), then remaining Base62 values.
    ///
    /// Total: 3843 values (excluding null "00"), with first 1295 being Base36.
    pub fn all_values() -> impl Iterator<Item = Self> {
        const BASE36_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        const BASE62_CHARS: &[u8] =
            b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

        // Generate all Base36 values first (1296 values: "00" to "ZZ")
        let base36_values = (0..36usize).flat_map(move |first_idx| {
            (0..36usize)
                .map(move |second_idx| Self([BASE36_CHARS[first_idx], BASE36_CHARS[second_idx]]))
        });

        // Generate all Base62 values, then filter out Base36 ones and "00"
        let remaining_values = (0..62usize).flat_map(move |first_idx| {
            (0..62usize)
                .map(move |second_idx| Self([BASE62_CHARS[first_idx], BASE62_CHARS[second_idx]]))
                .filter(move |obj_id| {
                    // Skip "00" and Base36 values (already yielded above)
                    !obj_id.is_null() && !obj_id.is_base36() && obj_id.is_base62()
                })
        });

        // Chain them: first Base36 (1296 values), then remaining (2548 values)
        // Total: 1296 + 2548 = 3844 values
        // Skip the first Base36 value ("00") to get 1295 Base36 + 2548 remaining = 3843 total
        base36_values.skip(1).chain(remaining_values)
    }
}

/// A play volume of the sound in the score. Defaults to 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Volume {
    /// A play volume percentage of the sound.
    pub relative_percent: u8,
}

impl Default for Volume {
    fn default() -> Self {
        Self {
            relative_percent: 100,
        }
    }
}

/// A POOR BGA display mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PoorMode {
    /// To hide the normal BGA and display the POOR BGA.
    #[default]
    Interrupt,
    /// To overlap the POOR BGA onto the normal BGA.
    Overlay,
    /// Not to display the POOR BGA.
    Hidden,
}

/// A notation type about LN in the score. But you don't have to take care of how the notes are actually placed in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LnType {
    /// The RDM type.
    #[default]
    Rdm,
    /// The MGQ type.
    Mgq,
}

/// Long Note Mode Type
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum LnMode {
    /// Normal Long Note, no tail judge (LN)
    #[default]
    Ln = 1,
    /// IIDX Classic Long Note, with tail judge (CN)
    Cn = 2,
    /// IIDX Hell Long Note, with tail judge. holding add gurge, un-holding lose gurge (HCN)
    Hcn = 3,
}

impl From<LnMode> for u8 {
    fn from(mode: LnMode) -> u8 {
        match mode {
            LnMode::Ln => 1,
            LnMode::Cn => 2,
            LnMode::Hcn => 3,
        }
    }
}

impl TryFrom<u8> for LnMode {
    type Error = u8;
    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::Ln,
            2 => Self::Cn,
            3 => Self::Hcn,
            _ => return Err(value),
        })
    }
}

/// Associates between object `K` and [`ObjId`] with memoization.
/// It is useful to assign object ids for many objects with its equality.
pub struct ObjIdManager<'a, K: ?Sized> {
    value_to_id: HashMap<&'a K, ObjId>,
    used_ids: HashSet<ObjId>,
    unused_ids: VecDeque<ObjId>,
}

impl<'a, K: ?Sized> Default for ObjIdManager<'a, K>
where
    K: std::hash::Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, K: ?Sized> ObjIdManager<'a, K>
where
    K: std::hash::Hash + Eq,
{
    /// Create a new empty ObjIdManager
    #[must_use]
    pub fn new() -> Self {
        let unused_ids: VecDeque<ObjId> = ObjId::all_values().collect();

        Self {
            value_to_id: HashMap::new(),
            used_ids: HashSet::new(),
            unused_ids,
        }
    }

    /// Create a new ObjIdManager with iterator of assigned entries
    pub fn from_entries<I: IntoIterator<Item = (&'a K, ObjId)>>(iter: I) -> Self {
        let mut value_to_id: HashMap<&'a K, ObjId> = HashMap::new();
        let mut used_ids: HashSet<ObjId> = HashSet::new();

        // Collect all entries first
        let entries: Vec<_> = iter.into_iter().collect();

        // Mark used IDs and build the mapping
        for (key, assigned_id) in entries {
            value_to_id.insert(key, assigned_id);
            used_ids.insert(assigned_id);
        }

        let unused_ids: VecDeque<ObjId> = ObjId::all_values()
            .filter(|id| !used_ids.contains(id))
            .collect();

        Self {
            value_to_id,
            used_ids,
            unused_ids,
        }
    }

    /// Returns whether the key is already assigned any id
    pub fn is_assigned(&self, key: &'a K) -> bool {
        self.value_to_id.contains_key(key)
    }

    /// Get or allocate an ObjId for a key without creating tokens
    pub fn get_or_new_id(&mut self, key: &'a K) -> ObjId {
        if let Some(&id) = self.value_to_id.get(key) {
            id
        } else if let Some(new_id) = self.unused_ids.pop_front() {
            self.used_ids.insert(new_id);
            self.value_to_id.insert(key, new_id);
            new_id
        } else {
            ObjId::null()
        }
    }

    /// Get used ids
    #[must_use]
    pub fn get_used_ids(&self) -> &HashSet<ObjId> {
        &self.used_ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base62() {
        assert_eq!(char_to_base62('/'), None);
        assert_eq!(char_to_base62('0'), Some(b'0'));
        assert_eq!(char_to_base62('9'), Some(b'9'));
        assert_eq!(char_to_base62(':'), None);
        assert_eq!(char_to_base62('@'), None);
        assert_eq!(char_to_base62('A'), Some(b'A'));
        assert_eq!(char_to_base62('Z'), Some(b'Z'));
        assert_eq!(char_to_base62('['), None);
        assert_eq!(char_to_base62('`'), None);
        assert_eq!(char_to_base62('a'), Some(b'a'));
        assert_eq!(char_to_base62('z'), Some(b'z'));
        assert_eq!(char_to_base62('{'), None);
    }

    #[test]
    fn test_all_values() {
        let all_values: Vec<ObjId> = ObjId::all_values().collect();

        // Should have exactly 3843 values
        assert_eq!(all_values.len(), 3843);

        // First 1295 values should be Base36 (0-9, A-Z)
        for (i, obj_id) in all_values.iter().enumerate() {
            if i < 1295 {
                assert!(
                    obj_id.is_base36(),
                    "Value at index {} should be Base36: {:?}",
                    i,
                    obj_id
                );
            } else {
                assert!(
                    !obj_id.is_base36(),
                    "Value at index {} should NOT be Base36: {:?}",
                    i,
                    obj_id
                );
            }
        }

        // Verify some specific values
        assert_eq!(all_values[0], ObjId::try_from(['0', '1']).unwrap()); // First Base36 value
        assert_eq!(all_values[1294], ObjId::try_from(['Z', 'Z']).unwrap()); // Last Base36 value

        // Verify that "00" is not included
        assert!(!all_values.contains(&ObjId::null()));

        // Verify that all values are unique
        let mut unique_values = std::collections::HashSet::new();
        for value in &all_values {
            assert!(
                unique_values.insert(*value),
                "Duplicate value found: {:?}",
                value
            );
        }
    }
}
