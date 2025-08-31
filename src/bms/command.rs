//! Definitions of command argument data.
//!
//! Structures in this module can be used in [Lex] part, [Parse] part, and the output models.

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
