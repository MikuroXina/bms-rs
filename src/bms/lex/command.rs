//! Definitions of command argument data.

use super::{LexError, Result, cursor::Cursor};

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

impl PlayerMode {
    pub(crate) fn from(c: &mut Cursor) -> Result<Self> {
        Ok(match c.next_token() {
            Some("1") => Self::Single,
            Some("2") => Self::Two,
            Some("3") => Self::Double,
            _ => return Err(c.make_err_expected_token("one of 1, 2 or 3")),
        })
    }
}

/// A rank to determine judge level, but treatment differs among the BMS players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
            _ => return Err(c.make_err_expected_token("one of 0, 1, 2 or 3")),
        })
    }
}

impl std::str::FromStr for JudgeLevel {
    type Err = LexError;
    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "0" => Self::VeryHard,
            "1" => Self::Hard,
            "2" => Self::Normal,
            "3" => Self::Easy,
            _ => {
                return Err(LexError::ExpectedToken {
                    line: 1,
                    col: 1,
                    message: "expected one of 0, 1, 2 or 3",
                });
            }
        })
    }
}

fn char_to_base62(ch: char) -> Result<u8> {
    match ch {
        '0'..='9' | 'A'..='Z' | 'a'..='z' => Ok(ch as u32 as u8),
        _ => Err(LexError::OutOfBase62),
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
    assert_eq!(char_to_base62('/'), Err(LexError::OutOfBase62));
    assert_eq!(char_to_base62('0'), Ok(b'0'));
    assert_eq!(char_to_base62('9'), Ok(b'9'));
    assert_eq!(char_to_base62(':'), Err(LexError::OutOfBase62));
    assert_eq!(char_to_base62('@'), Err(LexError::OutOfBase62));
    assert_eq!(char_to_base62('A'), Ok(b'A'));
    assert_eq!(char_to_base62('Z'), Ok(b'Z'));
    assert_eq!(char_to_base62('['), Err(LexError::OutOfBase62));
    assert_eq!(char_to_base62('`'), Err(LexError::OutOfBase62));
    assert_eq!(char_to_base62('a'), Ok(b'a'));
    assert_eq!(char_to_base62('z'), Ok(b'z'));
    assert_eq!(char_to_base62('{'), Err(LexError::OutOfBase62));
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

impl TryFrom<&str> for ObjId {
    type Error = LexError;
    fn try_from(value: &str) -> Result<Self> {
        if value.len() != 2 {
            return Err(LexError::ExpectedToken {
                line: 1,
                col: 1,
                message: "`0-9A-Za-z` was expected",
            });
        }
        let mut chars = value.chars();
        let ch1 = chars.next().unwrap();
        let ch2 = chars.next().unwrap();
        Ok(Self([char_to_base62(ch1)?, char_to_base62(ch2)?]))
    }
}

impl TryFrom<[char; 2]> for ObjId {
    type Error = LexError;
    fn try_from(value: [char; 2]) -> Result<Self> {
        Self::from_chars(value)
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

    /// Converts 2-digit of base-62 numeric characters into an object id.
    pub fn from_chars(chars: [char; 2]) -> Result<Self> {
        Ok(Self([char_to_base62(chars[0])?, char_to_base62(chars[1])?]))
    }

    pub(crate) fn from(id: &str, c: &mut Cursor) -> Result<Self> {
        id.try_into()
            .map_err(|_| c.make_err_expected_token("[0-9A-Za-z][0-9A-Za-z]"))
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

impl std::str::FromStr for Argb {
    type Err = LexError;
    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() != 4 {
            return Err(LexError::ExpectedToken {
                line: 1,
                col: 1,
                message: "expected 4 comma-separated values",
            });
        }
        let alpha = parts[0].parse().map_err(|_| LexError::ExpectedToken {
            line: 1,
            col: 1,
            message: "invalid alpha value",
        })?;
        let red = parts[1].parse().map_err(|_| LexError::ExpectedToken {
            line: 1,
            col: 1,
            message: "invalid red value",
        })?;
        let green = parts[2].parse().map_err(|_| LexError::ExpectedToken {
            line: 1,
            col: 1,
            message: "invalid green value",
        })?;
        let blue = parts[3].parse().map_err(|_| LexError::ExpectedToken {
            line: 1,
            col: 1,
            message: "invalid blue value",
        })?;
        Ok(Self {
            alpha,
            red,
            green,
            blue,
        })
    }
}

/// A kind of the note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl NoteKind {
    /// Returns whether the note is a playable.
    pub const fn is_playable(self) -> bool {
        !matches!(self, Self::Invisible)
    }

    /// Returns whether the note is a long-press note.
    pub const fn is_long(self) -> bool {
        matches!(self, Self::Long)
    }
}

/// A key of the controller or keyboard.
///
/// ```text
/// |---------|----------------------|
/// |         |   [K2]  [K4]  [K6]   |
/// |(Scratch)|[K1]  [K3]  [K5]  [K7]|
/// |---------|----------------------|
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Returns whether the key appears only 7 keys.
    pub fn is_extended_key(self) -> bool {
        matches!(self, Self::Key6 | Self::Key7)
    }

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
            _ => return Err(c.make_err_expected_token("[1-9]")),
        })
    }
}

/// A POOR BGA display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl PoorMode {
    pub(crate) fn from(c: &mut Cursor) -> Result<Self> {
        Ok(match c.next_token() {
            Some("0") => Self::Interrupt,
            Some("1") => Self::Overlay,
            Some("2") => Self::Hidden,
            _ => return Err(c.make_err_expected_token("one of 0, 1 or 2")),
        })
    }
}

/// The channel, or lane, where the note will be on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    SectionLen,
    /// For the stop object.
    Stop,
    /// For the scroll speed change object.
    Scroll,
    /// For the note spacing change object.
    Speed,
}

impl Channel {
    pub(crate) fn from(channel: &str, c: &mut Cursor) -> Result<Self> {
        use Channel::*;
        Ok(match channel.to_uppercase().as_str() {
            "01" => Bgm,
            "02" => SectionLen,
            "03" => BpmChangeU8,
            "08" => BpmChange,
            "04" => BgaBase,
            "06" => BgaPoor,
            "07" => BgaLayer,
            "09" => Stop,
            "SC" => Scroll,
            "SP" => Speed,
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
                });
            }
        })
    }
}

/// A track, or bar, in the score. It must greater than 0, but some scores may include the 0 track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Track(pub u32);
