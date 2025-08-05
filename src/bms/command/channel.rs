//! Definitions of channel command argument data.
//!
//! For more details, please see [`Channel`] enum and its related types.
//! For documents of modes, please see [BMS command memo#KEYMAP Table](https://hitkey.bms.ms/cmds.htm#KEYMAP-TABLE)
//!
//! - Pre-defined channel parsers:
//!   - `read_channel_beat` for Beat 5K/7K/10K/14K
//!   - `read_channel_pms_bme_type` for PMS BME-type
//!   - `read_channel_pms` for PMS
//!   - `read_channel_beat_nanasi` for Beat nanasi/angolmois
//!   - `read_channel_dsc_oct_fp` for DSC & OCT/FP

/// The channel, or lane, where the note will be on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
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
        /// The note for the player side.
        side: PlayerSide,
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
    /// For the video seek object. #SEEKxx n
    Seek,
    /// For the BGA LAYER2 object. #BMPxx (LAYER2 is layered over LAYER)
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
    /// For the TEXT object. #TEXTxx "string"
    Text,
    /// For the JUDGE object. #EXRANKxx n (100 corresponds to RANK:NORMAL. integer or decimal fraction)
    Judge,
    /// For the BGA BASE aRGB. #ARGBxx a,r,g,b (each [0-255])
    BgaBaseArgb,
    /// For the BGA LAYER aRGB. #ARGBxx
    BgaLayerArgb,
    /// For the BGA LAYER2 aRGB. #ARGBxx
    BgaLayer2Argb,
    /// For the BGA POOR aRGB. #ARGBxx
    BgaPoorArgb,
    /// For the BGA KEYBOUND. #SWBGAxx
    BgaKeybound,
    /// For the OPTION. #CHANGEOPTIONxx (multiline)
    Option,
}

/// Reads a channel from a string.
///
/// For general part, please call this function when using other functions.
fn read_channel_general(channel: &str) -> Option<Channel> {
    use Channel::*;
    Some(match channel.to_uppercase().as_str() {
        "01" => Bgm,
        "02" => SectionLen,
        "03" => BpmChangeU8,
        "08" => BpmChange,
        "04" => BgaBase,
        "05" => Seek,
        "06" => BgaPoor,
        "07" => BgaLayer,
        "09" => Stop,
        "0A" => BgaLayer2,
        "0B" => BgaBaseOpacity,
        "0C" => BgaLayerOpacity,
        "0D" => BgaLayer2Opacity,
        "0E" => BgaPoorOpacity,
        "97" => BgmVolume,
        "98" => KeyVolume,
        "99" => Text,
        "A0" => Judge,
        "A1" => BgaBaseArgb,
        "A2" => BgaLayerArgb,
        "A3" => BgaLayer2Argb,
        "A4" => BgaPoorArgb,
        "A5" => BgaKeybound,
        "A6" => Option,
        "SC" => Scroll,
        "SP" => Speed,
        _ => return None,
    })
}

/// Reads a note kind from a character. (For general part)
/// Can be directly use in BMS/BME/PMS types, and be converted to other types.
fn get_note_kind_general(kind_char: char) -> Option<(NoteKind, PlayerSide)> {
    Some(match kind_char {
        '1' => (NoteKind::Visible, PlayerSide::Player1),
        '2' => (NoteKind::Visible, PlayerSide::Player2),
        '3' => (NoteKind::Invisible, PlayerSide::Player1),
        '4' => (NoteKind::Invisible, PlayerSide::Player2),
        '5' => (NoteKind::Long, PlayerSide::Player1),
        '6' => (NoteKind::Long, PlayerSide::Player2),
        'D' => (NoteKind::Landmine, PlayerSide::Player1),
        'E' => (NoteKind::Landmine, PlayerSide::Player2),
        _ => return None,
    })
}

/// Reads a key from a character. (For Beat 5K/7K/10K/14K)
fn get_key_beat(key: char) -> Option<Key> {
    use Key::*;
    Some(match key {
        '1' => Key1,
        '2' => Key2,
        '3' => Key3,
        '4' => Key4,
        '5' => Key5,
        '6' => Scratch,
        '7' => FreeZone,
        '8' => Key6,
        '9' => Key7,
        _ => return None,
    })
}

/// Reads a channel from a string. (For Beat 5K/7K/10K/14K)
pub fn read_channel_beat(channel: &str) -> Option<Channel> {
    if let Some(channel) = read_channel_general(channel) {
        return Some(channel);
    }
    let mut channel_chars = channel.chars();
    let (kind, side) = get_note_kind_general(channel_chars.next()?)?;
    let key = get_key_beat(channel_chars.next()?)?;
    Some(Channel::Note { kind, side, key })
}

/// Reads a key from a character. (For PMS)
fn get_key_pms_bme_type(key: char) -> Option<Key> {
    use Key::*;
    Some(match key {
        '1' => Key1,
        '2' => Key2,
        '3' => Key3,
        '4' => Key4,
        '5' => Key5,
        '6' => Key8,
        '7' => Key9,
        '8' => Key6,
        '9' => Key7,
        _ => return None,
    })
}

/// Reads a channel from a string. (For PMS BME-type)
/// This preset supports 9K with 2-players.
pub fn read_channel_pms_bme_type(channel: &str) -> Option<Channel> {
    if let Some(channel) = read_channel_general(channel) {
        return Some(channel);
    }
    let mut channel_chars = channel.chars();
    let (kind, side) = get_note_kind_general(channel_chars.next()?)?;
    let key = get_key_pms_bme_type(channel_chars.next()?)?;
    Some(Channel::Note { kind, side, key })
}

/// Reads a channel from a string. (For PMS)
pub fn read_channel_pms(channel: &str) -> Option<Channel> {
    if let Some(channel) = read_channel_general(channel) {
        return Some(channel);
    }
    let mut channel_chars = channel.chars();
    let (kind, side) = get_note_kind_general(channel_chars.next()?)?;
    let bme_key = get_key_pms_bme_type(channel_chars.next()?)?;
    // Translate BME type to PMS type.
    use Key::*;
    use PlayerSide::*;
    let key = match (side, bme_key) {
        (Player1, Key1 | Key2 | Key3 | Key4 | Key5) => bme_key,
        (Player2, Key2) => Key6,
        (Player2, Key3) => Key7,
        (Player2, Key4) => Key8,
        (Player2, Key5) => Key9,
        _ => return None,
    };
    Some(Channel::Note {
        kind,
        side: PlayerSide::Player1,
        key,
    })
}

/// Reads a channel from a string. (For Beat nanasi/angolmois)
pub fn read_channel_beat_nanasi(channel: &str) -> Option<Channel> {
    if let Some(channel) = read_channel_general(channel) {
        return Some(channel);
    }
    let mut channel_chars = channel.chars();
    let (kind, side) = get_note_kind_general(channel_chars.next()?)?;
    let bme_key = get_key_beat(channel_chars.next()?)?;
    // Translate BME type to Beat nanasi/angolmois type.
    use Key::*;
    let key = match bme_key {
        Key1 | Key2 | Key3 | Key4 | Key5 | Scratch => bme_key,
        FreeZone => FootPedal,
        _ => return None,
    };
    Some(Channel::Note { kind, side, key })
}

/// Reads a channel from a string. (For DSC & OCT/FP)
pub fn read_channel_dsc_oct_fp(channel: &str) -> Option<Channel> {
    if let Some(channel) = read_channel_general(channel) {
        return Some(channel);
    }
    let mut channel_chars = channel.chars();
    let (kind, side) = get_note_kind_general(channel_chars.next()?)?;
    let bme_key = get_key_pms_bme_type(channel_chars.next()?)?;
    // Translate BME type to PMS type.
    use Key::*;
    use PlayerSide::*;
    let key = match (side, bme_key) {
        (Player1, Key1 | Key2 | Key3 | Key4 | Key5 | Key6 | Key7 | Scratch) => bme_key,
        (Player2, Key1) => FootPedal,
        (Player2, Key2) => Key8,
        (Player2, Key3) => Key9,
        (Player2, Key4) => Key10,
        (Player2, Key5) => Key11,
        (Player2, Key6) => Key12,
        (Player2, Key7) => Key13,
        (Player2, Scratch) => ScratchExtra,
        _ => return None,
    };
    Some(Channel::Note {
        kind,
        side: PlayerSide::Player1,
        key,
    })
}

/// A kind of the note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
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

/// A side of the player.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlayerSide {
    /// The player 1 side.
    #[default]
    Player1,
    /// The player 2 side.
    Player2,
}

/// A key of the controller or keyboard.
///
/// - Beat 5K/7K/10K/14K:
/// ```text
/// |---------|----------------------|
/// |         |   [K2]  [K4]  [K6]   |
/// |(Scratch)|[K1]  [K3]  [K5]  [K7]|
/// |---------|----------------------|
/// ```
///
/// - PMS:
/// ```text
/// |----------------------------|
/// |   [K2]  [K4]  [K6]  [K8]   |
/// |[K1]  [K3]  [K5]  [K7]  [K9]|
/// |----------------------------|
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Key {
    /// The leftmost white key.
    /// `11` in BME-type Player1.
    Key1,
    /// The leftmost black key.
    /// `12` in BME-type Player1.
    Key2,
    /// The second white key from the left.
    /// `13` in BME-type Player1.
    Key3,
    /// The second black key from the left.
    /// `14` in BME-type Player1.
    Key4,
    /// The third white key from the left.
    /// `15` in BME-type Player1.
    Key5,
    /// The rightmost black key.
    /// `18` in BME-type Player1.
    Key6,
    /// The rightmost white key.
    /// `19` in BME-type Player1.
    Key7,
    /// The extra black key. Used in PMS or other modes.
    Key8,
    /// The extra white key. Used in PMS or other modes.
    Key9,
    /// The extra key for OCT/FP.
    Key10,
    /// The extra key for OCT/FP.
    Key11,
    /// The extra key for OCT/FP.
    Key12,
    /// The extra key for OCT/FP.
    Key13,
    /// The extra key for OCT/FP.
    Key14,
    /// The scratch disk.
    /// `16` in BME-type Player1.
    Scratch,
    /// The extra scratch disk on the right. Used in DSC and OCT/FP mode.
    ScratchExtra,
    /// The foot pedal.
    FootPedal,
    /// The zone that the user can scratch disk freely.
    /// `17` in BMS-type Player1.
    FreeZone,
}
