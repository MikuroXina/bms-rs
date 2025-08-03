//! Definitions of command argument data.
//!
//! Structures in this module can be used in [Lex] part, [Parse] part, and the output models.

pub mod channel;
pub mod graphics;
pub mod time;

/// Minor command types and utilities.
/// 
/// This module contains types and utilities for minor BMS commands that are only available
/// when the `minor-command` feature is enabled.
#[cfg(feature = "minor-command")]
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
/// IIDX/LR2/beatoraja judge windows: https://iidx.org/misc/iidx_lr2_beatoraja_diff
///
/// Note: VeryEasy is not Implemented.
/// For `#RANK 4`, `#RANK 6` and `#RANK -1`: Usage differs among the BMS players.
/// See: https://github.com/MikuroXina/bms-rs/pull/122
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
    /// If used for ExRank, representing precentage.
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
        value
            .parse::<i64>()
            .map(JudgeLevel::from)
            .map_err(|_| value)
    }
}

fn char_to_base62(ch: char) -> Option<u8> {
    match ch {
        '0'..='9' | 'A'..='Z' | 'a'..='z' => Some(ch as u32 as u8),
        _ => None,
    }
}

fn base62_to_byte(base62: u8) -> u8 {
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
    pub const fn null() -> Self {
        Self([0, 0])
    }

    /// Converts the object id into an `u16` value.
    pub fn as_u16(self) -> u16 {
        self.into()
    }

    /// Converts the object id into an `u32` value.
    pub fn as_u32(self) -> u32 {
        self.into()
    }

    /// Converts the object id into an `u64` value.
    pub fn as_u64(self) -> u64 {
        self.into()
    }

    /// Makes the object id uppercase.
    pub fn make_uppercase(&mut self) {
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

/// An alpha-red-gree-blue color data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Argb {
    /// A component of alpha.
    pub alpha: u8,
    /// A component of red.
    pub red: u8,
    /// A component of green.
    pub green: u8,
    /// A component of blue.
    pub blue: u8,
}

impl Default for Argb {
    fn default() -> Self {
        Self {
            alpha: 255,
            red: 0,
            green: 0,
            blue: 0,
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



/// RGB struct, used for #VIDEOCOLORS and similar commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rgb {
    /// Red component
    pub r: u8,
    /// Green component
    pub g: u8,
    /// Blue component
    pub b: u8,
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
pub enum LnModeType {
    /// Normal Long Note (LN)
    #[default]
    Ln = 1,
    /// Classic Long Note (CN)
    Cn = 2,
    /// Hell Classic Long Note (HCN)
    Hcn = 3,
}

impl From<LnModeType> for u8 {
    fn from(mode: LnModeType) -> u8 {
        match mode {
            LnModeType::Ln => 1,
            LnModeType::Cn => 2,
            LnModeType::Hcn => 3,
        }
    }
}

impl TryFrom<u8> for LnModeType {
    type Error = u8;
    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            1 => LnModeType::Ln,
            2 => LnModeType::Cn,
            3 => LnModeType::Hcn,
            _ => return Err(value),
        })
    }
}



/// WAVCMD parameter type.
///
/// - Pitch: pitch (0-127, 60 is C6)
/// - Volume: volume percent (usually 0-100)
/// - Time: playback time (ms*0.5, 0 means original length)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WavCmdParam {
    /// Pitch (0-127, 60 is C6)
    Pitch,
    /// Volume percent (0-100 is recommended. Larger than 100 value is not recommended.)
    Volume,
    /// Playback time (ms*0.5, 0 means original length)
    Time,
}



/// BM98 #ExtChr extended character customization event.
///
/// Used for #ExtChr command, implements custom UI element image replacement.
/// - sprite_num: character index to replace [0-1023]
/// - bmp_num: BMP index (hex to decimal, or -1/-257, etc.)
/// - start_x/start_y: crop start point
/// - end_x/end_y: crop end point
/// - offset_x/offset_y: offset (optional)
/// - abs_x/abs_y: absolute coordinate (optional)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExtChrEvent {
    /// Character index to replace [0-1023]
    pub sprite_num: i32,
    /// BMP index (hex to decimal, or -1/-257, etc.)
    pub bmp_num: i32,
    /// Crop start point
    pub start_x: i32,
    /// Crop start point
    pub start_y: i32,
    /// Crop end point
    pub end_x: i32,
    /// Crop end point
    pub end_y: i32,
    /// Offset (optional)
    pub offset_x: Option<i32>,
    /// Offset (optional)
    pub offset_y: Option<i32>,
    /// Absolute coordinate (optional)
    pub abs_x: Option<i32>,
    /// Absolute coordinate (optional)
    pub abs_y: Option<i32>,
}


