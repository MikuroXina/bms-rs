//! Definitions of command argument data.
//!
//! Structures in this module can be used in [Lex] part, [Parse] part, and the output models.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    str::FromStr,
};

use fraction::BigDecimal;

use super::parse::ParseWarning;

pub mod channel;
pub mod graphics;
pub mod minor_command;
pub mod mixin;
pub mod time;

/// Represents a string that should be convert to a value by `FromStr`, and stores the result.
pub struct StringValue<T: FromStr> {
    /// The original string.
    string: String,
    /// The parsed value or the parsing error.
    value: Result<T, <T as FromStr>::Err>,
}

impl<T: FromStr> FromStr for StringValue<T> {
    type Err = <T as FromStr>::Err;
    fn from_str(str: &str) -> Result<Self, Self::Err> {
        let result = T::from_str(str);
        Ok(StringValue {
            string: str.to_string(),
            value: result,
        })
    }
}

impl<T: FromStr> StringValue<T> {
    /// Gets a reference to the original string.
    pub fn as_str(&self) -> &str {
        &self.string
    }

    /// Gets a reference to the parsed result.
    pub const fn parsed(&self) -> &Result<T, <T as FromStr>::Err> {
        &self.value
    }

    /// Checks if parsing succeeded.
    pub const fn is_ok(&self) -> bool {
        self.value.is_ok()
    }

    /// Checks if parsing failed.
    pub const fn is_err(&self) -> bool {
        self.value.is_err()
    }

    /// Creates `StringValue` from a value that implements `ToString`.
    #[must_use]
    pub fn from_value(value: T) -> Self
    where
        T: ToString,
    {
        let string = value.to_string();
        Self {
            string,
            value: Ok(value),
        }
    }

    /// Creates `StringValue` from a `Result<T, R>`.
    ///
    /// Allows creating `StringValue<T>` from any constructor that returns `Result<T, R>`,
    /// as long as the error type `R` can be converted to `<T as FromStr>::Err`.
    ///
    /// # Type parameters
    /// - `R`: The error type of the source Result, must implement `Into<<T as FromStr>::Err>`
    ///
    /// # Examples
    ///
    /// Create from `FinF64::new` (requires `FinF64`'s error type implements `Into<ParseError>`):
    ///
    /// ```text
    /// # use bms_rs::bms::command::StringValue;
    /// # use strict_num_extended::FinF64;
    /// let fin_result = FinF64::new(120.0);
    /// let sv = StringValue::from_result(fin_result);
    /// ```
    #[must_use]
    pub fn from_result<R>(result: Result<T, R>) -> Self
    where
        T: ToString,
        R: Into<<T as FromStr>::Err>,
    {
        match result {
            Ok(value) => {
                let string = value.to_string();
                Self {
                    string,
                    value: Ok(value),
                }
            }
            Err(err) => Self {
                string: String::new(),
                value: Err(err.into()),
            },
        }
    }
}

impl<T> Clone for StringValue<T>
where
    T: FromStr + Clone,
    <T as FromStr>::Err: Clone,
{
    fn clone(&self) -> Self {
        Self {
            string: self.string.clone(),
            value: self.value.clone(),
        }
    }
}

impl<T> std::fmt::Debug for StringValue<T>
where
    T: FromStr + std::fmt::Debug,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StringValue")
            .field("string", &self.string)
            .field("value", &self.value)
            .finish()
    }
}

impl<T> PartialEq for StringValue<T>
where
    T: FromStr,
{
    fn eq(&self, other: &Self) -> bool {
        self.string == other.string
    }
}

impl<T> Eq for StringValue<T> where T: FromStr {}

impl<T> std::hash::Hash for StringValue<T>
where
    T: FromStr + std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.string.hash(state);
    }
}

impl<T> PartialOrd for StringValue<T>
where
    T: FromStr,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for StringValue<T>
where
    T: FromStr,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.string.cmp(&other.string)
    }
}

impl<T> std::fmt::Display for StringValue<T>
where
    T: FromStr,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<T: FromStr> serde::Serialize for StringValue<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_str().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: FromStr> serde::Deserialize<'de> for StringValue<T>
where
    T::Err: std::fmt::Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        T::from_str(&string)
            .map(|v| StringValue {
                string,
                value: Ok(v),
            })
            .map_err(|e| serde::de::Error::custom(format!("Parse error: {}", e)))
    }
}

/// Represents a string slice that should be convert to a value by `FromStr`, and stores the result.
pub struct StrValue<'a, T: FromStr> {
    /// The original string slice.
    str_ref: &'a str,
    /// The parsed value or the parsing error.
    value: Result<T, <T as FromStr>::Err>,
}

impl<'a, T: FromStr> StrValue<'a, T> {
    /// Creates `StrValue` from string reference, parses and stores the result.
    #[must_use]
    pub fn parse(s: &'a str) -> Self {
        let result = T::from_str(s);
        StrValue {
            str_ref: s,
            value: result,
        }
    }

    /// Gets a reference to the original string slice.
    pub const fn as_str(&self) -> &str {
        self.str_ref
    }

    /// Gets a reference to the parsed result.
    pub const fn parsed(&self) -> &Result<T, <T as FromStr>::Err> {
        &self.value
    }

    /// Checks if parsing succeeded.
    pub const fn is_ok(&self) -> bool {
        self.value.is_ok()
    }

    /// Checks if parsing failed.
    pub const fn is_err(&self) -> bool {
        self.value.is_err()
    }
}

impl StringValue<strict_num_extended::FinF64> {
    /// Converts to f64 value for compatibility with existing code
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        self.value
            .as_ref()
            .ok()
            .map(strict_num_extended::FinF64::get)
    }

    /// Converts to u64 value for random number generation
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        self.value.as_ref().ok().map(|v| v.get() as u64)
    }

    /// Converts to `BigDecimal`
    #[must_use]
    pub fn as_big_decimal(&self) -> Option<BigDecimal> {
        self.as_f64().map(BigDecimal::from)
    }
}

impl StringValue<u64> {
    /// Gets the parsed u64 value
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        self.value.as_ref().ok().copied()
    }
}

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

impl std::fmt::Display for PlayerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single => write!(f, "1"),
            Self::Two => write!(f, "2"),
            Self::Double => write!(f, "3"),
        }
    }
}

impl std::str::FromStr for PlayerMode {
    type Err = ParseWarning;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "1" => Self::Single,
            "2" => Self::Two,
            "3" => Self::Double,
            _ => {
                return Err(ParseWarning::SyntaxError(
                    "expected one of 0, 1 or 2".into(),
                ));
            }
        })
    }
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

pub(crate) fn char_to_base62(ch: char) -> Option<u8> {
    ch.is_ascii_alphanumeric().then_some(ch as u8)
}

pub(crate) fn base62_to_byte(base62: u8) -> u8 {
    #[allow(clippy::panic)]
    match base62 {
        b'0'..=b'9' => base62 - b'0',
        b'A'..=b'Z' => base62 - b'A' + 10,
        b'a'..=b'z' => base62 - b'a' + 36,
        _ => panic!("invalid base62 byte: {base62}"),
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

impl std::str::FromStr for PoorMode {
    type Err = ParseWarning;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "0" => Self::Interrupt,
            "1" => Self::Overlay,
            "2" => Self::Hidden,
            _ => {
                return Err(ParseWarning::SyntaxError(
                    "expected one of 0, 1 or 2".into(),
                ));
            }
        })
    }
}

impl PoorMode {
    /// Converts an display type of Poor BGA into the corresponding string literal.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Interrupt => "0",
            Self::Overlay => "1",
            Self::Hidden => "2",
        }
    }
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
    fn from(mode: LnMode) -> Self {
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

impl<'a, K> Default for ObjIdManager<'a, K>
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
                "Duplicate value found: {:?}",
                value
            );
        }
    }
}
