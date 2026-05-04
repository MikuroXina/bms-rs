//! Definitions of channel command argument data.
//!
//! For more details, please see [`Channel`] enum and its related types.
//! For documents of modes, please see [BMS command memo#KEYMAP Table](https://hitkey.bms.ms/cmds.htm#KEYMAP-TABLE)
//!
//! For converting key/channel between different modes, please see `ModeKeyChannel` enum and `convert_key_channel_between` function.

use super::{base62_to_byte, char_to_base62};
use std::str::FromStr;
use thiserror::Error;

use self::mapper::BmsLayoutMapper;

// Import chart types for use in this module
use crate::chart::types::{Key, NoteKind, PlayerSide};

pub mod converter;
pub mod mapper;

/// The channel, or lane, where the note will be on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Channel {
    /// The BGA channel.
    BgaBase,
    /// The BGA channel but overlay to [`Channel::BgaBase`] channel.
    BgaLayer,
    /// The POOR BGA channel.
    BgaPoor,
    /// For the note which will be auto-played.
    Bgm,
    /// For the bpm change by an [`u8`] integer.
    BpmChangeU8,
    /// For the bpm change object.
    BpmChange,
    /// For the note which the user can interact.
    Note {
        /// The channel ID from the BMS file.
        channel_id: NoteChannelId,
    },
    /// For the section length change object.
    SectionLen,
    /// For the stop object.
    Stop,
    /// For the scroll speed change object.
    Scroll,
    /// For the note spacing change object.
    Speed,
    /// For the video seek object. `#SEEKxx n`
    Seek,
    /// For the BGA LAYER2 object. `#BMPxx` (LAYER2 is layered over LAYER)
    BgaLayer2,
    /// For the opacity of BGA BASE. transparent « [01-FF] » opaque
    BgaBaseOpacity,
    /// For the opacity of BGA LAYER. transparent « [01-FF] » opaque
    BgaLayerOpacity,
    /// For the opacity of BGA LAYER2. transparent « [01-FF] » opaque
    BgaLayer2Opacity,
    /// For the opacity of BGA POOR. transparent « [01-FF] » opaque
    BgaPoorOpacity,
    /// For the BGM volume. min 1 « [01-FF] » max 255 (= original sound)
    BgmVolume,
    /// For the KEY volume. min 1 « [01-FF] » max 255 (= original sound)
    KeyVolume,
    /// For the TEXT object. `#TEXTxx "string"`
    Text,
    /// For the JUDGE object. `#EXRANKxx n` (100 corresponds to RANK:NORMAL. integer or decimal fraction)
    Judge,
    /// For the BGA BASE aRGB. `#ARGBxx a,r,g,b` (each [0-255])
    BgaBaseArgb,
    /// For the BGA LAYER aRGB. `#ARGBxx`
    BgaLayerArgb,
    /// For the BGA LAYER2 aRGB. `#ARGBxx`
    BgaLayer2Argb,
    /// For the BGA POOR aRGB. `#ARGBxx`
    BgaPoorArgb,
    /// For the BGA KEYBOUND. `#SWBGAxx`
    BgaKeybound,
    /// For the OPTION. `#CHANGEOPTIONxx` (multiline)
    OptionChange,
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Channel: ")?;
        match self {
            Self::BgaBase => write!(f, "BGA"),
            Self::BgaLayer => write!(f, "BGA_LAYER"),
            Self::BgaPoor => write!(f, "BGA_POOR"),
            Self::Bgm => write!(f, "BGM"),
            Self::BpmChangeU8 => write!(f, "BPM_CHANGE_U8"),
            Self::BpmChange => write!(f, "BPM_CHANGE"),
            Self::Note { .. } => write!(f, "NOTE"),
            Self::SectionLen => write!(f, "SECTION_LEN"),
            Self::Stop => write!(f, "STOP"),
            Self::Scroll => write!(f, "SCROLL"),
            Self::Speed => write!(f, "SPEED"),

            Self::Seek => write!(f, "SEEK"),
            Self::BgaLayer2 => write!(f, "BGA_LAYER2"),

            Self::BgaBaseOpacity => write!(f, "BGA_BASE_OPACITY"),

            Self::BgaLayerOpacity => write!(f, "BGA_LAYER_OPACITY"),

            Self::BgaLayer2Opacity => write!(f, "BGA_LAYER2_OPACITY"),

            Self::BgaPoorOpacity => write!(f, "BGA_POOR_OPACITY"),
            Self::BgmVolume => write!(f, "BGM_VOLUME"),
            Self::KeyVolume => write!(f, "KEY_VOLUME"),
            Self::Text => write!(f, "TEXT"),
            Self::Judge => write!(f, "JUDGE"),

            Self::BgaBaseArgb => write!(f, "BGA_BASE_ARGB"),

            Self::BgaLayerArgb => write!(f, "BGA_LAYER_ARGB"),

            Self::BgaLayer2Argb => write!(f, "BGA_LAYER2_ARGB"),

            Self::BgaPoorArgb => write!(f, "BGA_POOR_ARGB"),

            Self::BgaKeybound => write!(f, "BGA_KEYBOUND"),

            Self::OptionChange => write!(f, "CHANGE_OPTION"),
        }
    }
}

/// Error type for parsing [`NoteChannelId`] from string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum ChannelIdParseError {
    /// The channel id must be exactly 2 ascii characters, got `{0}`.
    #[error("channel id must be exactly 2 ascii characters, got `{0}`")]
    ExpectedTwoAsciiChars(String),
    /// The channel id must be an alpha numeric to parse as base 62, got {0}.
    #[error("channel id must be an alpha numeric to parse as base 62, got {0}")]
    InvalidAsBase62(String),
}

/// A channel ID of notes playing sound.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NoteChannelId([u8; 2]);

impl std::fmt::Debug for NoteChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ChannelId")
            .field(&format!("{}{}", self.0[0] as char, self.0[1] as char))
            .finish()
    }
}

impl std::fmt::Display for NoteChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.0[0] as char, self.0[1] as char)
    }
}

impl TryFrom<[char; 2]> for NoteChannelId {
    type Error = [char; 2];
    fn try_from(value: [char; 2]) -> core::result::Result<Self, Self::Error> {
        Ok(Self([
            char_to_base62(value[0]).ok_or(value)?,
            char_to_base62(value[1]).ok_or(value)?,
        ]))
    }
}

impl TryFrom<[u8; 2]> for NoteChannelId {
    type Error = [u8; 2];
    fn try_from(value: [u8; 2]) -> core::result::Result<Self, Self::Error> {
        <Self as TryFrom<[char; 2]>>::try_from([value[0] as char, value[1] as char])
            .map_err(|_| value)
    }
}

impl FromStr for NoteChannelId {
    type Err = ChannelIdParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 2 {
            return Err(ChannelIdParseError::ExpectedTwoAsciiChars(s.to_string()));
        }
        let mut chars = s.bytes();
        let [Some(ch1), Some(ch2), None] = [chars.next(), chars.next(), chars.next()] else {
            return Err(ChannelIdParseError::ExpectedTwoAsciiChars(s.to_string()));
        };
        Self::try_from([ch1, ch2]).map_err(|_| ChannelIdParseError::InvalidAsBase62(s.to_string()))
    }
}

impl From<NoteChannelId> for u16 {
    fn from(value: NoteChannelId) -> Self {
        base62_to_byte(value.0[0]) as Self * 62 + base62_to_byte(value.0[1]) as Self
    }
}

impl From<NoteChannelId> for u32 {
    fn from(value: NoteChannelId) -> Self {
        Into::<u16>::into(value) as Self
    }
}

impl From<NoteChannelId> for u64 {
    fn from(value: NoteChannelId) -> Self {
        Into::<u16>::into(value) as Self
    }
}

impl NoteChannelId {
    /// Converts the channel id into an `u16` value.
    #[must_use]
    pub fn as_u16(self) -> u16 {
        self.into()
    }

    /// Gets a bgm channel ID.
    #[must_use]
    pub const fn bgm() -> Self {
        Self([b'0', b'1'])
    }

    /// Check if this is the BGM channel (`01`).
    #[must_use]
    pub const fn is_bgm(self) -> bool {
        self.0[0] == b'0' && self.0[1] == b'1'
    }

    /// Converts the channel into a key mapping.
    #[must_use]
    pub fn try_into_map<T: BmsLayoutMapper>(self) -> Option<T> {
        T::from_channel_id(self)
    }

    /// Check if this is a visible note channel (P1: first digit '1', P2: first digit '2').
    #[must_use]
    pub const fn is_visible_note_channel(self) -> bool {
        matches!(self.0[0], b'1' | b'2')
    }

    /// Convert a visible note channel to its long-note channel equivalent.
    ///
    /// P1 Visible (`1x`) → P1 Long (`5x`), P2 Visible (`2x`) → P2 Long (`6x`).
    /// Returns `None` if this is not a visible note channel.
    #[must_use]
    pub const fn to_long_note_channel(self) -> Option<Self> {
        if !self.is_visible_note_channel() {
            return None;
        }
        let long_first = if self.0[0] == b'1' { b'5' } else { b'6' };
        Some(Self([long_first, self.0[1]]))
    }
}

impl TryFrom<NoteChannelId> for Channel {
    type Error = NoteChannelId;

    fn try_from(channel_id: NoteChannelId) -> Result<Self, Self::Error> {
        match channel_id.0 {
            [b'0', b'1'] => Ok(Self::Bgm),
            [b'0', b'2'] => Ok(Self::SectionLen),
            [b'0', b'3'] => Ok(Self::BpmChangeU8),
            [b'0', b'4'] => Ok(Self::BgaBase),

            [b'0', b'5'] => Ok(Self::Seek),
            [b'0', b'6'] => Ok(Self::BgaPoor),
            [b'0', b'7'] => Ok(Self::BgaLayer),
            [b'0', b'8'] => Ok(Self::BpmChange),
            [b'0', b'9'] => Ok(Self::Stop),
            [b'0', b'A'] => Ok(Self::BgaLayer2),

            [b'0', b'B'] => Ok(Self::BgaBaseOpacity),

            [b'0', b'C'] => Ok(Self::BgaLayerOpacity),

            [b'0', b'D'] => Ok(Self::BgaLayer2Opacity),

            [b'0', b'E'] => Ok(Self::BgaPoorOpacity),
            [b'9', b'7'] => Ok(Self::BgmVolume),
            [b'9', b'8'] => Ok(Self::KeyVolume),
            [b'9', b'9'] => Ok(Self::Text),
            [b'A', b'0'] => Ok(Self::Judge),

            [b'A', b'1'] => Ok(Self::BgaBaseArgb),

            [b'A', b'2'] => Ok(Self::BgaLayerArgb),

            [b'A', b'3'] => Ok(Self::BgaLayer2Argb),

            [b'A', b'4'] => Ok(Self::BgaPoorArgb),

            [b'A', b'5'] => Ok(Self::BgaKeybound),

            [b'A', b'6'] => Ok(Self::OptionChange),
            [b'S', b'C'] => Ok(Self::Scroll),
            [b'S', b'P'] => Ok(Self::Speed),
            _ => Err(channel_id),
        }
    }
}

impl From<Channel> for NoteChannelId {
    fn from(channel: Channel) -> Self {
        match channel {
            Channel::Bgm => Self([b'0', b'1']),
            Channel::SectionLen => Self([b'0', b'2']),
            Channel::BpmChangeU8 => Self([b'0', b'3']),
            Channel::BgaBase => Self([b'0', b'4']),

            Channel::Seek => Self([b'0', b'5']),
            Channel::BgaPoor => Self([b'0', b'6']),
            Channel::BgaLayer => Self([b'0', b'7']),
            Channel::BpmChange => Self([b'0', b'8']),
            Channel::Stop => Self([b'0', b'9']),
            Channel::BgaLayer2 => Self([b'0', b'A']),

            Channel::BgaBaseOpacity => Self([b'0', b'B']),

            Channel::BgaLayerOpacity => Self([b'0', b'C']),

            Channel::BgaLayer2Opacity => Self([b'0', b'D']),

            Channel::BgaPoorOpacity => Self([b'0', b'E']),
            Channel::BgmVolume => Self([b'9', b'7']),
            Channel::KeyVolume => Self([b'9', b'8']),
            Channel::Text => Self([b'9', b'9']),
            Channel::Judge => Self([b'A', b'0']),

            Channel::BgaBaseArgb => Self([b'A', b'1']),

            Channel::BgaLayerArgb => Self([b'A', b'2']),

            Channel::BgaLayer2Argb => Self([b'A', b'3']),

            Channel::BgaPoorArgb => Self([b'A', b'4']),

            Channel::BgaKeybound => Self([b'A', b'5']),

            Channel::OptionChange => Self([b'A', b'6']),
            Channel::Scroll => Self([b'S', b'C']),
            Channel::Speed => Self([b'S', b'P']),
            Channel::Note { channel_id } => channel_id,
        }
    }
}

/// Reads a channel from a string.
///
/// For general part, please call this function when using other functions.
fn read_channel_general(channel: &str) -> Option<Channel> {
    let channel_id = channel.parse::<NoteChannelId>().ok()?;
    Channel::try_from(channel_id).ok()
}

/// Reads a channel from a string. (Generic channel reader)
#[must_use]
pub fn read_channel(channel: &str) -> Option<Channel> {
    if let Some(channel) = read_channel_general(channel) {
        return Some(channel);
    }
    let channel_id = channel.parse::<NoteChannelId>().ok()?;
    Some(Channel::Note { channel_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visible_to_long_p1() {
        let vis = NoteChannelId::try_from([b'1', b'1']).unwrap();
        assert!(vis.is_visible_note_channel());
        let long = vis.to_long_note_channel().unwrap();
        assert_eq!(long, NoteChannelId::try_from([b'5', b'1']).unwrap());
    }

    #[test]
    fn test_visible_to_long_p2() {
        let vis = NoteChannelId::try_from([b'2', b'1']).unwrap();
        assert!(vis.is_visible_note_channel());
        let long = vis.to_long_note_channel().unwrap();
        assert_eq!(long, NoteChannelId::try_from([b'6', b'1']).unwrap());
    }

    #[test]
    fn test_non_visible_returns_none() {
        let long = NoteChannelId::try_from([b'5', b'1']).unwrap();
        assert!(!long.is_visible_note_channel());
        assert!(long.to_long_note_channel().is_none());
    }

    #[test]
    fn test_invisible_not_visible() {
        let inv = NoteChannelId::try_from([b'3', b'1']).unwrap();
        assert!(!inv.is_visible_note_channel());
        assert!(inv.to_long_note_channel().is_none());
    }

    #[test]
    fn test_bgm_not_visible() {
        let bgm = NoteChannelId::bgm();
        assert!(!bgm.is_visible_note_channel());
    }

    #[test]
    fn test_landmine_not_visible() {
        let landmine = NoteChannelId::try_from([b'D', b'1']).unwrap();
        assert!(!landmine.is_visible_note_channel());
        assert!(landmine.to_long_note_channel().is_none());
    }

    #[test]
    fn test_bgm_is_bgm() {
        let bgm = NoteChannelId::bgm();
        assert!(bgm.is_bgm());
    }

    #[test]
    fn test_visible_not_bgm() {
        let vis = NoteChannelId::try_from([b'1', b'1']).unwrap();
        assert!(!vis.is_bgm());
    }

    #[test]
    fn test_long_not_bgm() {
        let long = NoteChannelId::try_from([b'5', b'1']).unwrap();
        assert!(!long.is_bgm());
    }

    #[test]
    fn test_invisible_not_bgm() {
        let inv = NoteChannelId::try_from([b'3', b'1']).unwrap();
        assert!(!inv.is_bgm());
    }
}
