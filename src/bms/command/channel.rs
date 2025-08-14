//! Definitions of channel command argument data.
//!
//! For more details, please see [`Channel`] enum and its related types.
//! For documents of modes, please see [BMS command memo#KEYMAP Table](https://hitkey.bms.ms/cmds.htm#KEYMAP-TABLE)
//!
//! For converting key/channel between different modes, please see [`ModeKeyChannel`] enum and [`convert_key_channel_between`] function.

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
    #[cfg(feature = "minor-command")]
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
    #[cfg(feature = "minor-command")]
    Seek,
    /// For the BGA LAYER2 object. #BMPxx (LAYER2 is layered over LAYER)
    BgaLayer2,
    /// For the opacity of BGA BASE. transparent « [01-FF] » opaque
    #[cfg(feature = "minor-command")]
    BgaBaseOpacity,
    /// For the opacity of BGA LAYER. transparent « [01-FF] » opaque
    #[cfg(feature = "minor-command")]
    BgaLayerOpacity,
    /// For the opacity of BGA LAYER2. transparent « [01-FF] » opaque
    #[cfg(feature = "minor-command")]
    BgaLayer2Opacity,
    /// For the opacity of BGA POOR. transparent « [01-FF] » opaque
    #[cfg(feature = "minor-command")]
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
    #[cfg(feature = "minor-command")]
    BgaBaseArgb,
    /// For the BGA LAYER aRGB. #ARGBxx
    #[cfg(feature = "minor-command")]
    BgaLayerArgb,
    /// For the BGA LAYER2 aRGB. #ARGBxx
    #[cfg(feature = "minor-command")]
    BgaLayer2Argb,
    /// For the BGA POOR aRGB. #ARGBxx
    #[cfg(feature = "minor-command")]
    BgaPoorArgb,
    /// For the BGA KEYBOUND. #SWBGAxx
    #[cfg(feature = "minor-command")]
    BgaKeybound,
    /// For the OPTION. #CHANGEOPTIONxx (multiline)
    #[cfg(feature = "minor-command")]
    Option,
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Channel: ")?;
        match self {
            Channel::BgaBase => write!(f, "BGA"),
            Channel::BgaLayer => write!(f, "BGA_LAYER"),
            Channel::BgaPoor => write!(f, "BGA_POOR"),
            Channel::Bgm => write!(f, "BGM"),
            Channel::BpmChangeU8 => write!(f, "BPM_CHANGE_U8"),
            Channel::BpmChange => write!(f, "BPM_CHANGE"),
            #[cfg(feature = "minor-command")]
            Channel::ChangeOption => write!(f, "CHANGE_OPTION"),
            Channel::Note { .. } => write!(f, "NOTE"),
            Channel::SectionLen => write!(f, "SECTION_LEN"),
            Channel::Stop => write!(f, "STOP"),
            Channel::Scroll => write!(f, "SCROLL"),
            Channel::Speed => write!(f, "SPEED"),
            #[cfg(feature = "minor-command")]
            Channel::Seek => write!(f, "SEEK"),
            Channel::BgaLayer2 => write!(f, "BGA_LAYER2"),
            #[cfg(feature = "minor-command")]
            Channel::BgaBaseOpacity => write!(f, "BGA_BASE_OPACITY"),
            #[cfg(feature = "minor-command")]
            Channel::BgaLayerOpacity => write!(f, "BGA_LAYER_OPACITY"),
            #[cfg(feature = "minor-command")]
            Channel::BgaLayer2Opacity => write!(f, "BGA_LAYER2_OPACITY"),
            #[cfg(feature = "minor-command")]
            Channel::BgaPoorOpacity => write!(f, "BGA_POOR_OPACITY"),
            Channel::BgmVolume => write!(f, "BGM_VOLUME"),
            Channel::KeyVolume => write!(f, "KEY_VOLUME"),
            Channel::Text => write!(f, "TEXT"),
            Channel::Judge => write!(f, "JUDGE"),
            #[cfg(feature = "minor-command")]
            Channel::BgaBaseArgb => write!(f, "BGA_BASE_ARGB"),
            #[cfg(feature = "minor-command")]
            Channel::BgaLayerArgb => write!(f, "BGA_LAYER_ARGB"),
            #[cfg(feature = "minor-command")]
            Channel::BgaLayer2Argb => write!(f, "BGA_LAYER2_ARGB"),
            #[cfg(feature = "minor-command")]
            Channel::BgaPoorArgb => write!(f, "BGA_POOR_ARGB"),
            #[cfg(feature = "minor-command")]
            Channel::BgaKeybound => write!(f, "BGA_KEYBOUND"),
            #[cfg(feature = "minor-command")]
            Channel::Option => write!(f, "OPTION"),
        }
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

/// A side of the player.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
#[repr(u64)]
pub enum Key {
    /// The leftmost white key.
    /// `11` in BME-type Player1.
    Key1 = 1,
    /// The leftmost black key.
    /// `12` in BME-type Player1.
    Key2 = 2,
    /// The second white key from the left.
    /// `13` in BME-type Player1.
    Key3 = 3,
    /// The second black key from the left.
    /// `14` in BME-type Player1.
    Key4 = 4,
    /// The third white key from the left.
    /// `15` in BME-type Player1.
    Key5 = 5,
    /// The rightmost black key.
    /// `18` in BME-type Player1.
    Key6 = 6,
    /// The rightmost white key.
    /// `19` in BME-type Player1.
    Key7 = 7,
    /// The extra black key. Used in PMS or other modes.
    Key8 = 8,
    /// The extra white key. Used in PMS or other modes.
    Key9 = 9,
    /// The extra key for OCT/FP.
    Key10 = 10,
    /// The extra key for OCT/FP.
    Key11 = 11,
    /// The extra key for OCT/FP.
    Key12 = 12,
    /// The extra key for OCT/FP.
    Key13 = 13,
    /// The extra key for OCT/FP.
    Key14 = 14,
    /// The scratch disk.
    /// `16` in BME-type Player1.
    Scratch = 101,
    /// The extra scratch disk on the right. Used in DSC and OCT/FP mode.
    ScratchExtra = 102,
    /// The foot pedal.
    FootPedal = 151,
    /// The zone that the user can scratch disk freely.
    /// `17` in BMS-type Player1.
    FreeZone = 201,
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
        #[cfg(feature = "minor-command")]
        "05" => Seek,
        "06" => BgaPoor,
        "07" => BgaLayer,
        "09" => Stop,
        "0A" => BgaLayer2,
        #[cfg(feature = "minor-command")]
        "0B" => BgaBaseOpacity,
        #[cfg(feature = "minor-command")]
        "0C" => BgaLayerOpacity,
        #[cfg(feature = "minor-command")]
        "0D" => BgaLayer2Opacity,
        #[cfg(feature = "minor-command")]
        "0E" => BgaPoorOpacity,
        "97" => BgmVolume,
        "98" => KeyVolume,
        "99" => Text,
        "A0" => Judge,
        #[cfg(feature = "minor-command")]
        "A1" => BgaBaseArgb,
        #[cfg(feature = "minor-command")]
        "A2" => BgaLayerArgb,
        #[cfg(feature = "minor-command")]
        "A3" => BgaLayer2Argb,
        #[cfg(feature = "minor-command")]
        "A4" => BgaPoorArgb,
        #[cfg(feature = "minor-command")]
        "A5" => BgaKeybound,
        #[cfg(feature = "minor-command")]
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

/// Trait for key channel mode implementations.
///
/// This trait defines the interface for converting between different key channel modes
/// and the standard Beat mode. Each mode implementation should provide methods to
/// convert from its own format to Beat format and vice versa.
pub trait KeyChannelMode {
    /// Convert from this mode's format to Beat mode format.
    ///
    /// This method takes a (PlayerSide, Key) pair in this mode's format and converts
    /// it to the equivalent BeatModeMap in Beat mode format.
    fn to_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap;

    /// Convert from Beat mode format to this mode's format.
    ///
    /// This method takes a BeatModeMap in Beat mode format and converts
    /// it to the equivalent (PlayerSide, Key) pair in this mode's format.
    fn map_from_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap;
}

/// Intermediate representation for Beat mode format.
///
/// This type represents a (PlayerSide, Key) pair in the standard Beat mode format,
/// which serves as the common intermediate representation for all key channel mode conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BeatModeMap(pub PlayerSide, pub Key);

impl BeatModeMap {
    /// Create a new BeatModeMap from a PlayerSide and Key.
    pub fn new(side: PlayerSide, key: Key) -> Self {
        BeatModeMap(side, key)
    }

    /// Get the PlayerSide from this BeatModeMap.
    pub fn side(&self) -> PlayerSide {
        self.0
    }

    /// Get the Key from this BeatModeMap.
    pub fn key(&self) -> Key {
        self.1
    }

    /// Deconstruct into a (PlayerSide, Key) tuple.
    pub fn into_tuple(self) -> (PlayerSide, Key) {
        (self.0, self.1)
    }
}

impl From<(PlayerSide, Key)> for BeatModeMap {
    fn from((side, key): (PlayerSide, Key)) -> Self {
        BeatModeMap::new(side, key)
    }
}

impl From<BeatModeMap> for (PlayerSide, Key) {
    fn from(beat_map: BeatModeMap) -> Self {
        beat_map.into_tuple()
    }
}

/// Beat 5K/7K/10K/14K, A mixture of BMS/BME type. (`16` is scratch, `17` is free zone)
/// It is the default type of key parsing.
///
/// - Lanes:
///   - Chars: '1'..'7','6' scratch, '7' free zone, '8'->Key6, '9'->Key7
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChannelModeBeat;

impl KeyChannelMode for KeyChannelModeBeat {
    fn to_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap {
        beat_map
    }

    fn map_from_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap {
        beat_map
    }
}

/// PMS BME-type, supports 9K/18K.
///
/// - Lanes:
///   - Chars: '1'..'9', '6'->Key8, '7'->Key9, '8'->Key6, '9'->Key7
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChannelModePmsBmeType;

impl KeyChannelMode for KeyChannelModePmsBmeType {
    fn to_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap {
        use Key::*;
        let side = beat_map.side();
        let key = match beat_map.key() {
            Key8 => Scratch,
            Key9 => FreeZone,
            other => other,
        };
        BeatModeMap::new(side, key)
    }

    fn map_from_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap {
        use Key::*;
        match beat_map.key() {
            Scratch => BeatModeMap::new(beat_map.side(), Key8),
            FreeZone => BeatModeMap::new(beat_map.side(), Key9),
            _ => beat_map,
        }
    }
}

/// PMS
///   
/// - Lanes:
///   - Beat -> this: (P2,Key2..Key5) remapped to (P1,Key6..Key9); (P1,Key1..Key5) unchanged
///   - This -> Beat: Key6..Key9 => (P2,Key2..Key5); Key1..Key5 => (P1,Key1..Key5)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChannelModePms;

impl KeyChannelMode for KeyChannelModePms {
    fn to_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap {
        use Key::*;
        use PlayerSide::*;
        match beat_map.into_tuple() {
            (Player1, Key1 | Key2 | Key3 | Key4 | Key5) => beat_map,
            (Player1, Key6) => BeatModeMap::new(Player2, Key2),
            (Player1, Key7) => BeatModeMap::new(Player2, Key3),
            (Player1, Key8) => BeatModeMap::new(Player2, Key4),
            (Player1, Key9) => BeatModeMap::new(Player2, Key5),
            other => other.into(),
        }
    }

    fn map_from_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap {
        use Key::*;
        use PlayerSide::*;
        match (beat_map.side(), beat_map.key()) {
            (Player1, Key1 | Key2 | Key3 | Key4 | Key5) => beat_map,
            (Player2, Key2) => BeatModeMap::new(Player1, Key6),
            (Player2, Key3) => BeatModeMap::new(Player1, Key7),
            (Player2, Key4) => BeatModeMap::new(Player1, Key8),
            (Player2, Key5) => BeatModeMap::new(Player1, Key9),
            other => other.into(),
        }
    }
}

/// Beat nanasi/angolmois
///
/// - Lanes:
///   - Beat -> this: FreeZone=>FootPedal
///   - This -> Beat: FootPedal=>FreeZone
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChannelModeBeatNanasi;

impl KeyChannelMode for KeyChannelModeBeatNanasi {
    fn to_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap {
        use Key::*;
        let key = match beat_map.key() {
            FootPedal => FreeZone,
            other => other,
        };
        BeatModeMap::new(beat_map.side(), key)
    }

    fn map_from_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap {
        use Key::*;
        let key = match beat_map.key() {
            FreeZone => FootPedal,
            other => other,
        };
        BeatModeMap::new(beat_map.side(), key)
    }
}

/// DSC & OCT/FP
///   
/// - Lanes:
///   - Beat -> this: (P2,Key1)=>FootPedal, (P2,Key2..Key7)=>Key8..Key13, (P2,Scratch)=>ScratchExtra; (P1,Key1..Key7|Scratch) unchanged; side becomes P1
///   - This -> Beat: reverse of above
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChannelModeDscOctFp;

impl KeyChannelMode for KeyChannelModeDscOctFp {
    fn to_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap {
        use Key::*;
        use PlayerSide::*;
        match beat_map.into_tuple() {
            (Player1, Key1 | Key2 | Key3 | Key4 | Key5 | Key6 | Key7 | Scratch) => beat_map,
            (Player1, ScratchExtra) => BeatModeMap::new(Player2, Scratch),
            (Player1, FootPedal) => BeatModeMap::new(Player2, Key1),
            (Player1, Key8) => BeatModeMap::new(Player2, Key2),
            (Player1, Key9) => BeatModeMap::new(Player2, Key3),
            (Player1, Key10) => BeatModeMap::new(Player2, Key4),
            (Player1, Key11) => BeatModeMap::new(Player2, Key5),
            (Player1, Key12) => BeatModeMap::new(Player2, Key6),
            (Player1, Key13) => BeatModeMap::new(Player2, Key7),
            (s, other) => BeatModeMap::new(s, other),
        }
    }

    fn map_from_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap {
        use Key::*;
        use PlayerSide::*;
        match (beat_map.side(), beat_map.key()) {
            (Player1, Key1 | Key2 | Key3 | Key4 | Key5 | Key6 | Key7 | Scratch) => beat_map,
            (Player2, Key1) => BeatModeMap::new(Player1, FootPedal),
            (Player2, Key2) => BeatModeMap::new(Player1, Key8),
            (Player2, Key3) => BeatModeMap::new(Player1, Key9),
            (Player2, Key4) => BeatModeMap::new(Player1, Key10),
            (Player2, Key5) => BeatModeMap::new(Player1, Key11),
            (Player2, Key6) => BeatModeMap::new(Player1, Key12),
            (Player2, Key7) => BeatModeMap::new(Player1, Key13),
            (Player2, Scratch) => BeatModeMap::new(Player1, ScratchExtra),
            (s, k) => BeatModeMap::new(s, k),
        }
    }
}

const KEY_DEFS: [Key; 14] = [
    Key::Key1,
    Key::Key2,
    Key::Key3,
    Key::Key4,
    Key::Key5,
    Key::Key6,
    Key::Key7,
    Key::Key8,
    Key::Key9,
    Key::Key10,
    Key::Key11,
    Key::Key12,
    Key::Key13,
    Key::Key14,
];

/// Mirror the note of player 1 side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChannelModeMirror {
    side: PlayerSide,
    key_count: usize,
}

impl KeyChannelMode for KeyChannelModeMirror {
    fn to_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap {
        self.map_from_beat(beat_map)
    }

    fn map_from_beat(&mut self, beat_map: BeatModeMap) -> BeatModeMap {
        let key_count = self.key_count;
        let (side, mut key) = beat_map.into_tuple();
        // Note: [`Key`] is 1-indexed, so we need to subtract 1 from the key count.
        if side == self.side && (key as u64 as usize) <= key_count {
            key = KEY_DEFS[key_count - (key as u64 as usize)];
        }
        BeatModeMap::new(side, key)
    }
}

/// Convert a key/channel between two different key channel modes.
///
/// This function takes two key channel modes and a (PlayerSide, Key) pair,
/// and converts it to the equivalent (PlayerSide, Key) pair in the destination mode.
pub fn convert_key_channel_between(
    src: &mut impl KeyChannelMode,
    dst: &mut impl KeyChannelMode,
    beat_map: BeatModeMap,
) -> BeatModeMap {
    let beat_map = src.to_beat(beat_map);
    dst.map_from_beat(beat_map)
}

#[cfg(test)]
mod channel_mode_tests {
    use super::*;

    #[test]
    fn test_key_channel_mode_mirror() {
        // Test 1: 3 keys
        let keys = vec![
            (PlayerSide::Player1, Key::Key1),
            (PlayerSide::Player1, Key::Key2),
            (PlayerSide::Player1, Key::Key3),
            (PlayerSide::Player1, Key::Key4),
            (PlayerSide::Player1, Key::Key5),
            (PlayerSide::Player2, Key::Key1),
            (PlayerSide::Player2, Key::Key2),
            (PlayerSide::Player2, Key::Key3),
            (PlayerSide::Player2, Key::Key4),
            (PlayerSide::Player2, Key::Key5),
        ]
        .into_iter()
        .map(|(side, key)| BeatModeMap::new(side, key))
        .collect::<Vec<_>>();
        let mut mode = KeyChannelModeMirror {
            side: PlayerSide::Player1,
            key_count: 3,
        };
        let expected = vec![
            (PlayerSide::Player1, Key::Key3),
            (PlayerSide::Player1, Key::Key2),
            (PlayerSide::Player1, Key::Key1),
            (PlayerSide::Player1, Key::Key4),
            (PlayerSide::Player1, Key::Key5),
            (PlayerSide::Player2, Key::Key1),
            (PlayerSide::Player2, Key::Key2),
            (PlayerSide::Player2, Key::Key3),
            (PlayerSide::Player2, Key::Key4),
            (PlayerSide::Player2, Key::Key5),
        ]
        .into_iter()
        .map(|(side, key)| BeatModeMap::new(side, key))
        .collect::<Vec<_>>();
        for (i, key) in keys.iter().enumerate() {
            let beat_map = mode.to_beat(*key);
            assert_eq!(beat_map, expected[i]);
            let beat_map = mode.map_from_beat(beat_map);
            assert_eq!(beat_map, *key);
        }
    }
}
