use std::num::NonZeroU16;

use crate::{cursor::Cursor, ParseError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerMode {
    Single,
    Two,
    Double,
}

impl PlayerMode {
    pub(crate) fn from(c: &mut Cursor) -> Result<Self> {
        Ok(match c.next_token() {
            Some("1") => Self::Single,
            Some("2") => Self::Two,
            Some("3") => Self::Double,
            _ => {
                return Err(ParseError::ExpectedToken {
                    line: c.line(),
                    col: c.col(),
                    message: "expected one of 1, 2 or 3",
                })
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JudgeLevel {
    VeryHard,
    Hard,
    Normal,
    Easy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WavId(NonZeroU16);

impl WavId {
    pub fn from(id: u16) -> Option<Self> {
        id.try_into().ok().map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BgiId(NonZeroU16);

impl BgiId {
    pub fn from(id: u16) -> Option<Self> {
        id.try_into().ok().map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Volume {
    pub relative_percent: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Argb {
    pub alpha: u8,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoteKind {
    Visible,
    Invisible,
    Long,
    Landmine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Scratch,
    FreeZone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpeedLength {
    integral: u64,
    fractional: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Channel {
    Bgm(WavId),
    SectionLen(SpeedLength),
    BpmChange(u8),
    BgaBase(BgiId),
    ExtObj(String),
    SeekObj(i32),
    BgaPoor(BgiId),
    BgaLayer(BgiId),
    ExtBpmChange(SpeedLength),
    Stop(u64),
    BgaLayer2(BgiId),
    BgaBaseOpacity(u8),
    BgaLayerOpactiy(u8),
    BgaLayer2Opacity(u8),
    BgaPoorOpacity(u8),
    BgmVolume(u8),
    KeyVolume(u8),
    Text(String),
    BgaBaseArgb(Argb),
    BgaLayerArgb(Argb),
    BgaLayer2Argb(Argb),
    BgaPoorArgb(Argb),
    BgaKeyBound(String),
    ChangeOption(String),
    Note {
        kind: NoteKind,
        is_player1: bool,
        key: Key,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Track(pub u32);
