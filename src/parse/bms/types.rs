//! Fundamental types for BMS format.
//!
//! This module contains core type definitions used across the BMS parsing
//! and processing pipeline. These types are format primitives:

//! - `ObjId`, `ObjIdManager`: Object identifier for WAV/BMP/BPM/STOP resources
//! - `ObjTime`, `Track`: Time representation in BMS
//! - `StringValue`: Value with preserved original string representation
//! - `JudgeLevel`: Difficulty/rank level

use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::NonZeroU64,
};

use num::Integer;

use super::parse::ParseWarning;

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

impl From<ObjId> for u16 {
    fn from(value: ObjId) -> Self {
        base62_to_byte(value.0[0]) as Self * 62 + base62_to_byte(value.0[1]) as Self
    }
}

impl From<ObjId> for u32 {
    fn from(value: ObjId) -> Self {
        Into::<u16>::into(value) as Self
    }
}

impl From<ObjId> for u64 {
    fn from(value: ObjId) -> Self {
        Into::<u16>::into(value) as Self
    }
}

impl ObjId {
    /// Instances a special null id, which means the rest object.
    #[must_use]
    pub const fn null() -> Self {
        Self([b'0', b'0'])
    }

    /// Returns whether the id is `00`.
    #[must_use]
    pub fn is_null(self) -> bool {
        self.0 == [b'0', b'0']
    }

    /// Parses the object id from the string `value`.
    ///
    /// If `case_sensitive_obj_id` is true, then the object id considered as a case-sensitive. Otherwise, it will be all uppercase characters.
    ///
    /// # Errors
    ///
    /// Returns [`ParseWarning::SyntaxError`] if `value` is not exactly two ASCII-alphanumeric characters.
    pub fn try_from(
        value: &str,
        case_sensitive_obj_id: bool,
    ) -> core::result::Result<Self, ParseWarning> {
        if value.len() != 2 {
            return Err(ParseWarning::SyntaxError(format!(
                "expected 2 digits as object id but found: {value}"
            )));
        }
        let mut chars = value.bytes();
        let [Some(ch1), Some(ch2), None] = [chars.next(), chars.next(), chars.next()] else {
            return Err(ParseWarning::SyntaxError(format!(
                "expected 2 digits as object id but found: {value}"
            )));
        };
        if !(ch1.is_ascii_alphanumeric() && ch2.is_ascii_alphanumeric()) {
            return Err(ParseWarning::SyntaxError(format!(
                "expected alphanumeric characters as object id but found: {value}"
            )));
        }
        if case_sensitive_obj_id {
            Ok(Self([ch1, ch2]))
        } else {
            Ok(Self([ch1.to_ascii_uppercase(), ch2.to_ascii_uppercase()]))
        }
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

    /// Converts the object id into 2 `char`s.
    #[must_use]
    pub fn into_chars(self) -> [char; 2] {
        self.0.map(|c| c as char)
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

    /// Returns an iterator over all possible `ObjId` values, ordered by priority:
    /// first all Base36 values (0-9, A-Z), then remaining Base62 values.
    ///
    /// Total: 3843 values (excluding null "00"), with first 1295 being Base36.
    pub fn all_values() -> impl Iterator<Item = Self> {
        const BASE36_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        const BASE62_CHARS: &[u8] =
            b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

        // Generate all Base36 values first (1296 values: "00" to "ZZ")
        let base36_values = BASE36_CHARS.iter().copied().flat_map(|first| {
            BASE36_CHARS
                .iter()
                .copied()
                .map(move |second| Self([first, second]))
        });

        // Generate all Base62 values, then filter out Base36 ones and "00"
        let remaining_values = BASE62_CHARS.iter().copied().flat_map(|first| {
            BASE62_CHARS
                .iter()
                .copied()
                .map(move |second| Self([first, second]))
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

pub(crate) fn char_to_base62(ch: char) -> Option<u8> {
    ch.is_ascii_alphanumeric().then_some(ch as u8)
}

pub(crate) fn base62_to_byte(base62: u8) -> u8 {
    #[expect(clippy::panic)]
    match base62 {
        b'0'..=b'9' => base62 - b'0',
        b'A'..=b'Z' => base62 - b'A' + 10,
        b'a'..=b'z' => base62 - b'a' + 36,
        _ => panic!("invalid base62 byte: {base62}"),
    }
}

/// Associates between object `K` and [`ObjId`] with memoization.
/// It is useful to assign object ids for many objects with its equality.
pub struct ObjIdManager<'a, K: ?Sized> {
    value_to_id: HashMap<&'a K, ObjId>,
    used_ids: HashSet<ObjId>,
    unused_ids: VecDeque<ObjId>,
}

impl<K> Default for ObjIdManager<'_, K>
where
    K: std::hash::Hash + Eq + ?Sized,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, K> ObjIdManager<'a, K>
where
    K: std::hash::Hash + Eq + ?Sized,
{
    /// Creates a new empty `ObjIdManager`.
    #[must_use]
    pub fn new() -> Self {
        let unused_ids: VecDeque<ObjId> = ObjId::all_values().collect();

        Self {
            value_to_id: HashMap::new(),
            used_ids: HashSet::new(),
            unused_ids,
        }
    }

    /// Creates a new `ObjIdManager` with iterator of assigned entries.
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

    /// Returns whether the key is already assigned any id.
    pub fn is_assigned(&self, key: &'a K) -> bool {
        self.value_to_id.contains_key(key)
    }

    /// Gets or allocates an `ObjId` for a key without creating tokens.
    pub fn get_or_new_id(&mut self, key: &'a K) -> Option<ObjId> {
        if let Some(&id) = self.value_to_id.get(key) {
            return Some(id);
        }

        let new_id = self.unused_ids.pop_front()?;
        self.used_ids.insert(new_id);
        self.value_to_id.insert(key, new_id);
        Some(new_id)
    }

    /// Get assigned ids as an iterator.
    pub fn into_assigned_ids(self) -> impl Iterator<Item = ObjId> {
        self.used_ids.into_iter()
    }
}

// ===== Time types =====

/// A track, or measure, or bar, in the score. It must greater than 0, but some scores may include the 0 track, where the object is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Track(pub u64);

impl Track {
    /// Returns the contained track number.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Track: {:03}", self.0)
    }
}

impl From<Track> for u64 {
    fn from(track: Track) -> Self {
        track.0
    }
}

impl From<u64> for Track {
    fn from(track: u64) -> Self {
        Self(track)
    }
}

/// A time of the object on the score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjTime {
    /// The track, or measure, where the object is in.
    track: Track,
    /// The time offset numerator in the track.
    numerator: u64,
    /// The time offset denominator in the track.
    denominator: NonZeroU64,
}

impl ObjTime {
    /// Create a new time from raw u64 values.
    /// Returns `None` if the denominator is zero.
    #[must_use]
    pub fn new(track: u64, numerator: u64, denominator: u64) -> Option<Self> {
        let denominator = NonZeroU64::new(denominator)?;
        Some(Self::new_checked(track, numerator, denominator))
    }

    /// Create a new time from a guaranteed non-zero denominator.
    ///
    /// # Panics
    ///
    /// Panics if the reduced denominator would be zero (which cannot happen with a non-zero input).
    #[must_use]
    pub fn new_checked(track: u64, numerator: u64, denominator: NonZeroU64) -> Self {
        // If numerator is greater than denominator, add the integer part of numerator / denominator to track and set numerator to the remainder.
        let (track, numerator) = if numerator > denominator.get() {
            (
                track + (numerator / denominator.get()),
                numerator % denominator.get(),
            )
        } else {
            (track, numerator)
        };
        // Reduce the fraction to the simplest form.
        // Note: 0.gcd(&num) == num, when num > 0
        let gcd = numerator.gcd(&denominator.get());
        let reduced_denominator = denominator.get() / gcd;
        Self {
            track: Track(track),
            numerator: numerator / gcd,
            denominator: NonZeroU64::new(reduced_denominator)
                .expect("reduced denominator must be non-zero"),
        }
    }

    /// Create a new time from a track.
    #[must_use]
    pub const fn start_of(track: Track) -> Self {
        Self {
            track,
            numerator: 0,
            denominator: NonZeroU64::MIN,
        }
    }

    /// Get the track where the object is in.
    #[must_use]
    pub const fn track(&self) -> Track {
        self.track
    }

    /// Get the time offset numerator in the track.
    #[must_use]
    pub const fn numerator(&self) -> u64 {
        self.numerator
    }

    /// Get the time offset denominator in the track.
    #[must_use]
    pub const fn denominator(&self) -> NonZeroU64 {
        self.denominator
    }

    /// Get the time offset denominator in the track as u64.
    #[must_use]
    pub const fn denominator_u64(&self) -> u64 {
        self.denominator.get()
    }
}

impl PartialOrd for ObjTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ObjTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_time_in_track = self.numerator() * other.denominator().get();
        let other_time_in_track = other.numerator() * self.denominator().get();
        self.track()
            .cmp(&other.track())
            .then(self_time_in_track.cmp(&other_time_in_track))
    }
}

impl std::fmt::Display for ObjTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ObjTime: {}, {} / {}",
            self.track(),
            self.numerator(),
            self.denominator().get()
        )
    }
}

// ===== StringValue =====

use std::fmt::Debug;
use std::str::FromStr;

/// Type alias for the error type of a `FromStr` implementation.
type ParseError<T> = <T as FromStr>::Err;

/// A wrapper that preserves the original string representation of a value.
/// The value is stored as a Result to preserve parsing errors.
pub struct StringValue<T: FromStr>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    raw: String,
    value: Result<T, ParseError<T>>,
}

impl<T> StringValue<T>
where
    T: FromStr,
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    /// Creates a `StringValue` from a value, generating a string representation.
    pub fn from_value(value: T) -> Self
    where
        T: ToString,
    {
        Self {
            raw: value.to_string(),
            value: Ok(value),
        }
    }

    /// Creates a `StringValue` from a raw string.
    #[must_use]
    pub fn new(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let value = T::from_str(&raw);
        Self { raw, value }
    }

    /// Returns a reference to the parsed value.
    pub const fn value(&self) -> &Result<T, ParseError<T>> {
        &self.value
    }

    /// Returns a reference to the raw string.
    pub fn raw(&self) -> &str {
        &self.raw
    }
}

// Implement Debug manually
impl<T: Debug + FromStr> Debug for StringValue<T>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StringValue")
            .field("raw", &self.raw)
            .field("value", &self.value)
            .finish()
    }
}

// Implement Clone manually
impl<T: FromStr + Clone> Clone for StringValue<T>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            value: self.value.clone(),
        }
    }
}

// Implement PartialEq manually
impl<T: FromStr + PartialEq> PartialEq for StringValue<T>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw && self.value == other.value
    }
}

// Implement Eq manually - only when T implements Eq
impl<T: FromStr + Eq> Eq for StringValue<T> where ParseError<T>: Debug + Clone + PartialEq + Eq {}

impl<T> AsRef<Result<T, ParseError<T>>> for StringValue<T>
where
    T: FromStr,
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn as_ref(&self) -> &Result<T, ParseError<T>> {
        &self.value
    }
}

impl<T: std::fmt::Display + FromStr> std::fmt::Display for StringValue<T>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.raw, f)
    }
}

// Serde support - manually implement
#[cfg(feature = "serde")]
impl<T: FromStr> serde::Serialize for StringValue<T>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.raw.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: FromStr> serde::Deserialize<'de> for StringValue<T>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(Self::new(raw))
    }
}

// ===== JudgeLevel =====

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
                    "Value at index {i} should be Base36: {obj_id:?}"
                );
            } else {
                assert!(
                    !obj_id.is_base36(),
                    "Value at index {i} should NOT be Base36: {obj_id:?}"
                );
            }
        }

        // Verify some specific values
        let Some(first) = all_values.first().copied() else {
            panic!("expected ObjId::all_values() to be non-empty");
        };
        assert_eq!(first, ObjId::try_from("01", false).unwrap()); // First Base36 value
        let Some(last_base36) = all_values.get(1294).copied() else {
            panic!("expected ObjId::all_values() to contain Base36 values");
        };
        assert_eq!(last_base36, ObjId::try_from("ZZ", false).unwrap()); // Last Base36 value

        // Verify that "00" is not included
        assert!(!all_values.contains(&ObjId::null()));

        // Verify that all values are unique
        let mut unique_values = std::collections::HashSet::new();
        for value in &all_values {
            assert!(
                unique_values.insert(*value),
                "Duplicate value found: {value:?}"
            );
        }
    }
}
