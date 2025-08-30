//! Definitions of channel command argument data.
//!
//! For more details, please see [`Channel`] enum and its related types.
//! For documents of modes, please see [BMS command memo#KEYMAP Table](https://hitkey.bms.ms/cmds.htm#KEYMAP-TABLE)
//!
//! For converting key/channel between different modes, please see [`ModeKeyChannel`] enum and [`convert_key_channel_between`] function.

pub mod converter;
pub mod mapper;

// mapper imports only when needed

/// A logical note channel (lane), represented in base62 two-digit encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NoteChannel(pub [u8; 2]);

impl NoteChannel {
    /// Construct from two ASCII characters (both in base62 range).
    pub fn try_from_chars(c1: char, c2: char) -> Option<Self> {
        fn char_to_digit(ch: char) -> Option<u8> {
            let ch = ch as u8;
            match ch {
                b'0'..=b'9' => Some(ch - b'0'),
                b'A'..=b'Z' => Some(ch - b'A' + 10),
                b'a'..=b'z' => Some(ch - b'a' + 36),
                _ => None,
            }
        }
        // Normalize to base62 digits (0-61) for storage
        let d1 = char_to_digit(c1)?;
        let d2 = char_to_digit(c2)?;
        Some(Self([d1, d2]))
    }

    /// Construct from "YY" two-character string.
    pub fn try_from_str(s: &str) -> Option<Self> {
        if s.len() != 2 {
            return None;
        }
        let mut it = s.chars();
        let c1 = it.next()?;
        let c2 = it.next()?;
        Self::try_from_chars(c1, c2)
    }

    /// Encode internal base62 digit pair to u16 (d1*62 + d2).
    pub const fn to_u16(self) -> u16 {
        (self.0[0] as u16) * 62 + (self.0[1] as u16)
    }

    /// Construct from u16 (less than 62*62).
    pub fn from_u16(v: u16) -> Self {
        let hi = (v / 62) as u8;
        let lo = (v % 62) as u8;
        Self([hi, lo])
    }

    /// Get display string (two characters, using standard base62 character set).
    pub fn to_str(self) -> [char; 2] {
        // Reverse map base62 digits to characters
        fn digit_to_char(d: u8) -> char {
            match d {
                0..=9 => (b'0' + d) as char,
                10..=35 => (b'A' + (d - 10)) as char,
                36..=61 => (b'a' + (d - 36)) as char,
                _ => unreachable!(),
            }
        }
        [digit_to_char(self.0[0]), digit_to_char(self.0[1])]
    }
}

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
        /// The logical note lane channel.
        channel: NoteChannel,
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

impl Channel {
    /// Returns the two-character code used in BMS commands for this channel.
    pub fn to_bms_code(&self) -> [char; 2] {
        use Channel::*;
        match self {
            BgaBase => ['0', '4'],
            BgaLayer => ['0', '7'],
            BgaPoor => ['0', '6'],
            Bgm => ['0', '1'],
            BpmChangeU8 => ['0', '3'],
            BpmChange => ['0', '8'],
            #[cfg(feature = "minor-command")]
            ChangeOption => ['A', '6'],
            Note { channel, .. } => channel.to_str(),
            SectionLen => ['0', '2'],
            Stop => ['0', '9'],
            Scroll => ['S', 'C'],
            Speed => ['S', 'P'],
            #[cfg(feature = "minor-command")]
            Seek => ['0', '5'],
            BgaLayer2 => ['0', 'A'],
            #[cfg(feature = "minor-command")]
            BgaBaseOpacity => ['0', 'B'],
            #[cfg(feature = "minor-command")]
            BgaLayerOpacity => ['0', 'C'],
            #[cfg(feature = "minor-command")]
            BgaLayer2Opacity => ['0', 'D'],
            #[cfg(feature = "minor-command")]
            BgaPoorOpacity => ['0', 'E'],
            BgmVolume => ['9', '7'],
            KeyVolume => ['9', '8'],
            Text => ['9', '9'],
            Judge => ['A', '0'],
            #[cfg(feature = "minor-command")]
            BgaBaseArgb => ['A', '1'],
            #[cfg(feature = "minor-command")]
            BgaLayerArgb => ['A', '2'],
            #[cfg(feature = "minor-command")]
            BgaLayer2Argb => ['A', '3'],
            #[cfg(feature = "minor-command")]
            BgaPoorArgb => ['A', '4'],
            #[cfg(feature = "minor-command")]
            BgaKeybound => ['A', '5'],
            #[cfg(feature = "minor-command")]
            Option => ['A', '6'],
        }
    }
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

    /// Derive NoteKind from a logical NoteChannel.
    /// Default rule by YY first character: 1/2 Visible, 3/4 Invisible, 5/6 Long, D/E Landmine.
    pub fn note_kind_from_channel(channel: NoteChannel) -> Option<Self> {
        let [c1, _] = channel.to_str();
        Some(match c1.to_ascii_uppercase() {
            '1' | '2' => NoteKind::Visible,
            '3' | '4' => NoteKind::Invisible,
            '5' | '6' => NoteKind::Long,
            'D' | 'E' => NoteKind::Landmine,
            _ => return None,
        })
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
    /// Returns whether the key expected a piano keyboard.
    pub const fn is_keyxx(&self) -> bool {
        use Key::*;
        matches!(
            self,
            Key1 | Key2
                | Key3
                | Key4
                | Key5
                | Key6
                | Key7
                | Key8
                | Key9
                | Key10
                | Key11
                | Key12
                | Key13
                | Key14
        )
    }
}

/// Reads a channel from a string.
///
/// For general part, please call this function when using other functions.
pub fn read_channel_general(channel: &str) -> Option<Channel> {
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

// removed redundant router helpers; kind is inferred at parse stage

// removed redundant beat key parser; mapping now centralized in PhysicalKey impls

// No longer provides unified wrapper routing function, caller directly uses NoteChannel::try_from_str or read_channel_general.

// read_channel_beat has been removed, unified to use `read_channel` (returns NoteChannel) and `read_channel_general` (general enum only).
