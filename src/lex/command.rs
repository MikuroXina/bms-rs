use std::num::NonZeroU16;

use super::{cursor::Cursor, LexError, Result};

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
            _ => return Err(c.err_expected_token("one of 1, 2 or 3")),
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

impl JudgeLevel {
    pub(crate) fn from(c: &mut Cursor) -> Result<Self> {
        Ok(match c.next_token() {
            Some("0") => Self::VeryHard,
            Some("1") => Self::Hard,
            Some("2") => Self::Normal,
            Some("3") => Self::Easy,
            _ => return Err(c.err_expected_token("one of 0, 1, 2 or 3")),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjId(pub NonZeroU16);

impl TryFrom<u16> for ObjId {
    type Error = std::num::TryFromIntError;

    fn try_from(value: u16) -> std::result::Result<Self, Self::Error> {
        Ok(Self(value.try_into()?))
    }
}

impl ObjId {
    pub(crate) fn from(id: &str, c: &mut Cursor) -> Result<Self> {
        let id = u16::from_str_radix(id, 36).map_err(|_| c.err_expected_token("[00-ZZ]"))?;
        id.try_into()
            .map(Self)
            .map_err(|_| c.err_expected_token("non zero index"))
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

impl Key {
    pub(crate) fn from(key: &str, c: &mut Cursor) -> Result<Self> {
        use Key::*;
        Ok(match key {
            "1" => Key1,
            "2" => Key2,
            "3" => Key3,
            "4" => Key4,
            "5" => Key5,
            "6" => Scratch,
            "7" => FreeZone,
            "8" => Key6,
            "9" => Key7,
            _ => return Err(c.err_expected_token("[1-9]")),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]

pub enum PoorMode {
    Interrupt,
    Overlay,
    Hidden,
}

impl Default for PoorMode {
    fn default() -> Self {
        Self::Interrupt
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Channel {
    BgaBase,
    BgaLayer,
    BgaPoor,
    Bgm,
    BpmChange,
    ChangeOption,
    Note {
        kind: NoteKind,
        is_player1: bool,
        key: Key,
    },
    SectionLen,
    Stop,
}

impl Channel {
    pub(crate) fn from(channel: &str, c: &mut Cursor) -> Result<Self> {
        use Channel::*;
        Ok(match channel.to_uppercase().as_str() {
            "01" => Bgm,
            "02" => SectionLen,
            "03" | "08" => BpmChange,
            "04" => BgaBase,
            "06" => BgaPoor,
            "07" => BgaLayer,
            "09" => Stop,
            player1 if player1.starts_with('1') => Note {
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::from(&channel[1..], c)?,
            },
            player2 if player2.starts_with('2') => Note {
                kind: NoteKind::Visible,
                is_player1: false,
                key: Key::from(&channel[1..], c)?,
            },
            player1 if player1.starts_with('3') => Note {
                kind: NoteKind::Invisible,
                is_player1: true,
                key: Key::from(&channel[1..], c)?,
            },
            player2 if player2.starts_with('4') => Note {
                kind: NoteKind::Invisible,
                is_player1: false,
                key: Key::from(&channel[1..], c)?,
            },
            player1 if player1.starts_with('5') => Note {
                kind: NoteKind::Long,
                is_player1: true,
                key: Key::from(&channel[1..], c)?,
            },
            player2 if player2.starts_with('6') => Note {
                kind: NoteKind::Long,
                is_player1: false,
                key: Key::from(&channel[1..], c)?,
            },
            player1 if player1.starts_with('D') => Note {
                kind: NoteKind::Landmine,
                is_player1: true,
                key: Key::from(&channel[1..], c)?,
            },
            player2 if player2.starts_with('E') => Note {
                kind: NoteKind::Landmine,
                is_player1: false,
                key: Key::from(&channel[1..], c)?,
            },
            _ => {
                return Err(LexError::UnknownCommand {
                    line: c.line(),
                    col: c.col(),
                })
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Track(pub u32);