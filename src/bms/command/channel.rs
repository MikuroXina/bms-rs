//! Definitions of channel command argument data.
//!
//! For more details, please see [`Channel`] enum and its related types.
//! For documents of modes, please see [BMS command memo#KEYMAP Table](https://hitkey.bms.ms/cmds.htm#KEYMAP-TABLE)
//!
//! For converting key/channel between different modes, please see [`ModeKeyChannel`] enum and [`convert_key_channel_between`] function.

use std::collections::HashMap;

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

impl Key {
    /// Returns whether the key is a keyboard key.
    pub const fn is_in_keypad(&self) -> bool {
        matches!(
            self,
            Self::Key1
                | Self::Key2
                | Self::Key3
                | Self::Key4
                | Self::Key5
                | Self::Key6
                | Self::Key7
                | Self::Key8
                | Self::Key9
                | Self::Key10
                | Self::Key11
                | Self::Key12
                | Self::Key13
                | Self::Key14
        )
    }
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
pub trait KeyLayoutMapper {
    /// Convert from this mode's format to Beat mode format.
    ///
    /// This method takes a (PlayerSide, Key) pair in this mode's format and converts
    /// it to the equivalent BeatModeMap in Beat mode format.
    fn to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping;

    /// Convert from Beat mode format to this mode's format.
    ///
    /// This method takes a BeatModeMap in Beat mode format and converts
    /// it to the equivalent (PlayerSide, Key) pair in this mode's format.
    fn map_from_beat(&mut self, beat_map: KeyMapping) -> KeyMapping;
}

/// Intermediate representation for Beat mode format.
///
/// This type represents a (PlayerSide, Key) pair in the standard Beat mode format,
/// which serves as the common intermediate representation for all key channel mode conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyMapping(pub PlayerSide, pub Key);

impl KeyMapping {
    /// Create a new BeatModeMap from a PlayerSide and Key.
    pub fn new(side: PlayerSide, key: Key) -> Self {
        KeyMapping(side, key)
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

impl From<(PlayerSide, Key)> for KeyMapping {
    fn from((side, key): (PlayerSide, Key)) -> Self {
        KeyMapping::new(side, key)
    }
}

impl From<KeyMapping> for (PlayerSide, Key) {
    fn from(beat_map: KeyMapping) -> Self {
        beat_map.into_tuple()
    }
}

/// Convert a key/channel between two different key channel modes.
///
/// This function takes two key channel modes and a (PlayerSide, Key) pair,
/// and converts it to the equivalent (PlayerSide, Key) pair in the destination mode.
pub fn convert_key_mapping_between(
    src: &mut impl KeyLayoutMapper,
    dst: &mut impl KeyLayoutMapper,
    beat_map: KeyMapping,
) -> KeyMapping {
    let beat_map = src.to_beat(beat_map);
    dst.map_from_beat(beat_map)
}

/// Beat 5K/7K/10K/14K, A mixture of BMS/BME type. (`16` is scratch, `17` is free zone)
/// It is the default type of key parsing.
///
/// - Lanes:
///   - Chars: '1'..'7','6' scratch, '7' free zone, '8'->Key6, '9'->Key7
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyLayoutBeat;

impl KeyLayoutMapper for KeyLayoutBeat {
    fn to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        beat_map
    }

    fn map_from_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        beat_map
    }
}

/// PMS BME-type, supports 9K/18K.
///
/// - Lanes:
///   - Chars: '1'..'9', '6'->Key8, '7'->Key9, '8'->Key6, '9'->Key7
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyLayoutPmsBmeType;

impl KeyLayoutMapper for KeyLayoutPmsBmeType {
    fn to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        use Key::*;
        let side = beat_map.side();
        let key = match beat_map.key() {
            Key8 => Scratch,
            Key9 => FreeZone,
            other => other,
        };
        KeyMapping::new(side, key)
    }

    fn map_from_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        use Key::*;
        match beat_map.key() {
            Scratch => KeyMapping::new(beat_map.side(), Key8),
            FreeZone => KeyMapping::new(beat_map.side(), Key9),
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
pub struct KeyLayoutPms;

impl KeyLayoutMapper for KeyLayoutPms {
    fn to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        use Key::*;
        use PlayerSide::*;
        match beat_map.into_tuple() {
            (Player1, Key1 | Key2 | Key3 | Key4 | Key5) => beat_map,
            (Player1, Key6) => KeyMapping::new(Player2, Key2),
            (Player1, Key7) => KeyMapping::new(Player2, Key3),
            (Player1, Key8) => KeyMapping::new(Player2, Key4),
            (Player1, Key9) => KeyMapping::new(Player2, Key5),
            other => other.into(),
        }
    }

    fn map_from_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        use Key::*;
        use PlayerSide::*;
        match (beat_map.side(), beat_map.key()) {
            (Player1, Key1 | Key2 | Key3 | Key4 | Key5) => beat_map,
            (Player2, Key2) => KeyMapping::new(Player1, Key6),
            (Player2, Key3) => KeyMapping::new(Player1, Key7),
            (Player2, Key4) => KeyMapping::new(Player1, Key8),
            (Player2, Key5) => KeyMapping::new(Player1, Key9),
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
pub struct KeyLayoutBeatNanasi;

impl KeyLayoutMapper for KeyLayoutBeatNanasi {
    fn to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        use Key::*;
        let key = match beat_map.key() {
            FootPedal => FreeZone,
            other => other,
        };
        KeyMapping::new(beat_map.side(), key)
    }

    fn map_from_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        use Key::*;
        let key = match beat_map.key() {
            FreeZone => FootPedal,
            other => other,
        };
        KeyMapping::new(beat_map.side(), key)
    }
}

/// DSC & OCT/FP
///   
/// - Lanes:
///   - Beat -> this: (P2,Key1)=>FootPedal, (P2,Key2..Key7)=>Key8..Key13, (P2,Scratch)=>ScratchExtra; (P1,Key1..Key7|Scratch) unchanged; side becomes P1
///   - This -> Beat: reverse of above
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyLayoutDscOctFp;

impl KeyLayoutMapper for KeyLayoutDscOctFp {
    fn to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        use Key::*;
        use PlayerSide::*;
        match beat_map.into_tuple() {
            (Player1, Key1 | Key2 | Key3 | Key4 | Key5 | Key6 | Key7 | Scratch) => beat_map,
            (Player1, ScratchExtra) => KeyMapping::new(Player2, Scratch),
            (Player1, FootPedal) => KeyMapping::new(Player2, Key1),
            (Player1, Key8) => KeyMapping::new(Player2, Key2),
            (Player1, Key9) => KeyMapping::new(Player2, Key3),
            (Player1, Key10) => KeyMapping::new(Player2, Key4),
            (Player1, Key11) => KeyMapping::new(Player2, Key5),
            (Player1, Key12) => KeyMapping::new(Player2, Key6),
            (Player1, Key13) => KeyMapping::new(Player2, Key7),
            (s, other) => KeyMapping::new(s, other),
        }
    }

    fn map_from_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        use Key::*;
        use PlayerSide::*;
        match (beat_map.side(), beat_map.key()) {
            (Player1, Key1 | Key2 | Key3 | Key4 | Key5 | Key6 | Key7 | Scratch) => beat_map,
            (Player2, Key1) => KeyMapping::new(Player1, FootPedal),
            (Player2, Key2) => KeyMapping::new(Player1, Key8),
            (Player2, Key3) => KeyMapping::new(Player1, Key9),
            (Player2, Key4) => KeyMapping::new(Player1, Key10),
            (Player2, Key5) => KeyMapping::new(Player1, Key11),
            (Player2, Key6) => KeyMapping::new(Player1, Key12),
            (Player2, Key7) => KeyMapping::new(Player1, Key13),
            (Player2, Scratch) => KeyMapping::new(Player1, ScratchExtra),
            (s, k) => KeyMapping::new(s, k),
        }
    }
}

/// Mirror the note of a [`PlayerSide`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyChannelModeMirror {
    /// The side of the player to mirror.
    side: PlayerSide,
    /// A list of [`Key`]s to mirror. Usually, it should be the keys that actually used in the song.
    keys: Vec<Key>,
}

impl KeyChannelModeMirror {
    /// Create a new [`KeyChannelModeMirror`] with the given [`PlayerSide`] and [`Key`]s.
    pub fn new(side: PlayerSide, keys: Vec<Key>) -> Self {
        Self { side, keys }
    }
}

impl KeyLayoutMapper for KeyChannelModeMirror {
    fn to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        beat_map
    }

    fn map_from_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        let (side, mut key) = beat_map.into_tuple();
        if side == self.side
            && let Some(position) = self.keys.iter().position(|k| k == &key)
        {
            let mirror_index = self.keys.len().saturating_sub(position + 1);
            let Some(mirror_key) = self.keys.get(mirror_index) else {
                return beat_map;
            };
            key = *mirror_key;
        }
        KeyMapping::new(side, key)
    }
}

/// A random number generator based on Java's `java.util.Random`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    /// Create a new [`JavaRandom`] with the given seed.
    pub fn new(seed: i64) -> Self {
        let s = (seed as u64) ^ 0x5DEECE66D;
        JavaRandom {
            seed: s & ((1u64 << 48) - 1),
        }
    }

    /// Java's next(int bits) method
    fn next(&mut self, bits: i32) -> i32 {
        const MULT: u64 = 0x5DEECE66D;
        const ADD: u64 = 0xB;
        self.seed = (self.seed.wrapping_mul(MULT).wrapping_add(ADD)) & ((1u64 << 48) - 1);
        ((self.seed >> (48 - bits)) & ((1u64 << bits) - 1)) as i32
    }

    /// Java's nextInt() method - returns any int value
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// Java's nextInt(int bound) method
    pub fn next_int_bound(&mut self, bound: i32) -> i32 {
        if bound <= 0 {
            panic!("bound must be positive");
        }

        let m = bound - 1;
        if (bound & m) == 0 {
            // i.e., bound is a power of 2
            ((bound as i64 * self.next(31) as i64) >> 31) as i32
        } else {
            loop {
                let bits = self.next(31);
                let val = bits % bound;
                if bits - val + m >= 0 {
                    return val;
                }
            }
        }
    }
}

/// A modifier that rotates the lanes of a [`KeyChannelMode`].
#[derive(Debug, Clone)]
pub struct LaneRotateShuffleModifier {
    /// The side of the player to shuffle.
    side: PlayerSide,
    /// A map of [`Key`]s to their new [`Key`]s.
    arrangement: HashMap<Key, Key>,
}

impl LaneRotateShuffleModifier {
    /// Create a new [`LaneRotateShuffleModifier`] with the given [`PlayerSide`], [`Key`]s and seed.
    pub fn new(side: PlayerSide, keys: Vec<Key>, seed: i64) -> Self {
        LaneRotateShuffleModifier {
            side,
            arrangement: Self::make_random(&keys, seed),
        }
    }

    fn make_random(keys: &[Key], seed: i64) -> HashMap<Key, Key> {
        let mut rng = JavaRandom::new(seed);
        let mut result: HashMap<Key, Key> = HashMap::new();
        if keys.is_empty() {
            return result;
        }

        let inc = rng.next_int_bound(2) == 1;
        let start = rng.next_int_bound(keys.len() as i32 - 1) as usize + if inc { 1 } else { 0 };

        let mut rlane = start;
        for lane in 0..keys.len() {
            result.insert(keys[lane], keys[rlane]);
            rlane = if inc {
                (rlane + 1) % keys.len()
            } else {
                (rlane + keys.len() - 1) % keys.len()
            };
        }
        result
    }
}

impl KeyLayoutMapper for LaneRotateShuffleModifier {
    fn to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        beat_map
    }

    fn map_from_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        let (side, key) = beat_map.into_tuple();
        if side == self.side {
            let new_key = self.arrangement.get(&key).copied().unwrap_or(key);
            KeyMapping::new(side, new_key)
        } else {
            beat_map
        }
    }
}

/// A modifier that shuffles the lanes of a [`KeyChannelMode`].
///
/// Its action is similar to beatoraja's lane shuffle.
#[derive(Debug, Clone)]
pub struct LaneRandomShuffleModifier {
    /// The side of the player to shuffle.
    side: PlayerSide,
    /// A map of [`Key`]s to their new [`Key`]s.
    arrangement: HashMap<Key, Key>,
}

impl LaneRandomShuffleModifier {
    /// Create a new [`LaneRandomShuffleModifier`] with the given [`PlayerSide`], [`Key`]s and seed.
    pub fn new(side: PlayerSide, keys: Vec<Key>, seed: i64) -> Self {
        LaneRandomShuffleModifier {
            side,
            arrangement: Self::make_random(&keys, seed),
        }
    }

    fn make_random(keys: &[Key], seed: i64) -> HashMap<Key, Key> {
        let mut rng = JavaRandom::new(seed);
        let mut result: HashMap<Key, Key> = HashMap::new();
        if keys.is_empty() {
            return result;
        }

        let mut l = keys.to_vec();
        for &lane in keys {
            let r = rng.next_int_bound(l.len() as i32) as usize;
            result.insert(lane, l[r]);
            l.remove(r);
        }

        result
    }
}

impl KeyLayoutMapper for LaneRandomShuffleModifier {
    fn to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        beat_map
    }

    fn map_from_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
        let (side, key) = beat_map.into_tuple();
        if side == self.side {
            let new_key = self.arrangement.get(&key).copied().unwrap_or(key);
            KeyMapping::new(side, new_key)
        } else {
            beat_map
        }
    }
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
        .map(|(side, key)| KeyMapping::new(side, key))
        .collect::<Vec<_>>();
        let mut mode = KeyChannelModeMirror {
            side: PlayerSide::Player1,
            keys: vec![Key::Key1, Key::Key2, Key::Key3],
        };
        let result = keys
            .iter()
            .map(|k| mode.map_from_beat(*k))
            .collect::<Vec<_>>();
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
        .map(|(side, key)| KeyMapping::new(side, key))
        .collect::<Vec<_>>();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_java_random_consistency() {
        // Test with seed 123456789
        let mut rng = JavaRandom::new(123456789);

        // Test nextInt() method (returns any int value)
        println!("First nextInt(): {}", rng.next_int());
        println!("Second nextInt(): {}", rng.next_int());
        println!("Third nextInt(): {}", rng.next_int());

        // Test nextInt(bound) method
        let mut rng2 = JavaRandom::new(123456789);
        println!("First nextInt(100): {}", rng2.next_int_bound(100));
        println!("Second nextInt(100): {}", rng2.next_int_bound(100));
        println!("Third nextInt(100): {}", rng2.next_int_bound(100));

        // Basic functionality test - should not panic
        assert!(rng2.next_int_bound(100) >= 0 && rng2.next_int_bound(100) < 100);
    }

    /// Test the random shuffle modifier.
    ///
    /// Source: https://www.bilibili.com/opus/1033281595747860483
    #[test]
    fn test_random_shuffle() {
        let examples = [
            "1234567 4752",
            "1234576 2498",
            "4372615 12728",
            "4372651 9734",
            "4375126 139",
        ]
        .iter()
        .map(|s| {
            let v = s.split_whitespace().collect::<Vec<_>>();
            let [list, seed] = v.as_slice() else {
                println!("{:?}", v);
                panic!("Invalid input");
            };
            let list = list
                .chars()
                .map(|c| c.to_digit(10).unwrap() as usize)
                .collect::<Vec<_>>();
            let seed = seed.parse::<i64>().unwrap();
            (list, seed)
        })
        .collect::<Vec<_>>();

        for (i, (list, seed)) in examples.iter().enumerate() {
            println!("Test case {}: seed = {}", i, seed);
            let init_keys = [
                Key::Key1,
                Key::Key2,
                Key::Key3,
                Key::Key4,
                Key::Key5,
                Key::Key6,
                Key::Key7,
            ];
            let mut rnd =
                LaneRandomShuffleModifier::new(PlayerSide::Player1, init_keys.to_vec(), *seed);
            let result_values = init_keys
                .into_iter()
                .map(|k| rnd.map_from_beat(KeyMapping::new(PlayerSide::Player1, k)))
                .map(|v| v.key() as usize)
                .collect::<Vec<_>>();
            println!("  Expected: {:?}", list);
            println!("  Got:      {:?}", result_values);
            println!("  Match:    {}", result_values == *list);
            if result_values != *list {
                println!("  FAILED!");
            }
            println!();
        }
    }
}
