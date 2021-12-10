//! Definitions of command argument data.

use std::num::NonZeroU16;

use super::{cursor::Cursor, LexError, Result};

/// A play style of the score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerMode {
    /// For single play, a player uses 5 or 7 keys.
    Single,
    /// For couple play, two players use each 5 or 7 keys.
    Two,
    /// For double play, a player uses 10 or 14 keys.
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

/// A rank to determine judge level, but treatment differs among the BMS players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JudgeLevel {
    /// Rank 0, the most difficult rank.
    VeryHard,
    /// Rank 1, the harder rank.
    Hard,
    /// Rank 2, the easier rank.
    Normal,
    /// Rank 3, the easiest rank.
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

/// An object id. Its meaning is determined by the channel belonged to.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjId(pub NonZeroU16);

impl std::fmt::Debug for ObjId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let digits = (self.0.get() / 36, self.0.get() % 36);
        f.debug_tuple("ObjId")
            .field(&format!(
                "{}{}",
                char::from_digit(digits.0 as u32, 36).unwrap(),
                char::from_digit(digits.1 as u32, 36).unwrap()
            ))
            .finish()
    }
}

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

/// A play volume of the sound in the score. Defaults to 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// A kind of the note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoteKind {
    /// A normal note can be seen by the user.
    Visible,
    /// A invisible note cannot be played by the user.
    Invisible,
    /// A long-press note (LN), requires the user to hold pressing the key.
    Long,
    /// A landmine note that treated as POOR judgement when
    Landmine,
}

/// A key of the controller or keyboard.
///
/// |---------|----------------------|
/// |         |   [K2]  [K4]  [K6]   |
/// |(Scratch)|[K1]  [K3]  [K5]  [K7]|
/// |---------|----------------------|
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// The leftmost white key.
    Key1,
    /// The leftmost black key.
    Key2,
    /// The second white key from the left.
    Key3,
    /// The second black key from the left.
    Key4,
    /// The third white key from the left.
    Key5,
    /// The rightmost black key.
    Key6,
    /// The rightmost white key.
    Key7,
    /// The scratch disk.
    Scratch,
    /// The zone that the user can scratch disk freely.
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

/// A POOR BGA display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]

pub enum PoorMode {
    /// To hide the normal BGA and display the POOR BGA.
    Interrupt,
    /// To overlap the POOR BGA onto the normal BGA.
    Overlay,
    /// Not to display the POOR BGA.
    Hidden,
}

impl Default for PoorMode {
    fn default() -> Self {
        Self::Interrupt
    }
}

/// The channel, or lane, where the note will be on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Channel {
    /// The BGA channel.
    BgaBase,
    /// The BGA channel but overlay to [`Channel::BgaBase`] channel.
    BgaLayer,
    /// The POOR BGA channel.
    BgaPoor,
    /// For the note which will be auto-played.
    Bgm,
    /// For the bpm change object.
    BpmChange,
    /// For the change option object.
    ChangeOption,
    /// For the note which the user can interact.
    Note {
        /// The kind of the note.
        kind: NoteKind,
        /// The note for the player 1.
        is_player1: bool,
        /// The key which corresponds to the note.
        key: Key,
    },
    /// For the section length change object.
    SectionLen(String),
    /// For the stop object.
    Stop,
}

impl Channel {
    pub(crate) fn from(channel: &str, c: &mut Cursor) -> Result<Self> {
        use Channel::*;
        Ok(match channel.to_uppercase().as_str() {
            "01" => Bgm,
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

/// A track, or bar, in the score. It must greater than 0, but some scores may include the 0 track.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Track(pub u32);
