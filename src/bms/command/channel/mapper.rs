//! Mappers between modes

use super::{Key, NoteChannel, NoteKind, PlayerSide};

/// A trait for key mapping storage structure.
pub trait KeyMapping {
    /// Create a new [`KeyMapping`] from a [`PlayerSide`], [`Key`] and [`NoteKind`].
    fn new(side: PlayerSide, key: Key, kind: NoteKind) -> Self;
    /// Create a new [`KeyMapping`] from a [`PlayerSide`] and [`Key`] with default [`NoteKind`].
    fn new_with_default_kind(side: PlayerSide, key: Key) -> Self
    where
        Self: Sized,
    {
        Self::new(side, key, NoteKind::Visible)
    }
    /// Get the PlayerSide from this KeyMapping.
    fn side(&self) -> PlayerSide;
    /// Get the [`Key`] from this [`KeyMapping`].
    fn key(&self) -> Key;
    /// Get the [`NoteKind`] from this [`KeyMapping`].
    fn kind(&self) -> NoteKind;
    /// Deconstruct into a [`PlayerSide`], [`Key`], [`NoteKind`] tuple.
    fn into_tuple(self) -> (PlayerSide, Key, NoteKind);
}

/// A trait that maps between logical [`NoteChannel`] and concrete physical key layouts.
pub trait PhysicalKey: Copy + Eq + core::fmt::Debug {
    /// Convert this physical key into a logical note channel.
    fn to_note_channel(self) -> NoteChannel;
    /// Convert a logical note channel into this physical key, if representable.
    fn from_note_channel(channel: NoteChannel) -> Option<Self>;
}

/// Default Beat layout physical key (encapsulating side and key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BeatKey {
    /// The side of the player.
    pub side: PlayerSide,
    /// The key of the player.
    pub key: Key,
    /// The kind of note.
    pub kind: NoteKind,
}

impl BeatKey {
    /// Create a new [`BeatKey`] from a [`PlayerSide`], [`Key`] and [`NoteKind`].
    pub const fn new(side: PlayerSide, key: Key, kind: NoteKind) -> Self {
        Self { side, key, kind }
    }

    /// Create a new [`BeatKey`] from a [`PlayerSide`] and [`Key`] with default [`NoteKind`].
    pub const fn new_with_default_kind(side: PlayerSide, key: Key) -> Self {
        Self::new(side, key, NoteKind::Visible)
    }
}

impl Default for BeatKey {
    fn default() -> Self {
        Self {
            side: PlayerSide::default(),
            key: Key::Key1,
            kind: NoteKind::Visible,
        }
    }
}

impl KeyMapping for BeatKey {
    fn new(side: PlayerSide, key: Key, kind: NoteKind) -> Self {
        Self::new(side, key, kind)
    }
    fn side(&self) -> PlayerSide {
        self.side
    }
    fn key(&self) -> Key {
        self.key
    }
    fn kind(&self) -> NoteKind {
        self.kind
    }
    fn into_tuple(self) -> (PlayerSide, Key, NoteKind) {
        (self.side, self.key, self.kind)
    }
}

impl PhysicalKey for BeatKey {
    fn to_note_channel(self) -> NoteChannel {
        // Map side and key to base62 two-character:
        // First character: side (using visible note default: P1='1', P2='2')
        // Second character: key code ('1'..'5', '6'=Scratch, '7'=FreeZone, '8'=Key6/'Key8, '9'=Key7/'Key9,
        //                      'A'..'E'=Key10..Key14, 'F'=FootPedal, '7'=ScratchExtra)
        use Key::*;
        let side_char = match self.side {
            PlayerSide::Player1 => '1',
            PlayerSide::Player2 => '2',
        };
        let key_char = match self.key {
            Key1 => '1',
            Key2 => '2',
            Key3 => '3',
            Key4 => '4',
            Key5 => '5',
            Key6 => '8',
            Key7 => '9',
            Key8 => '8',
            Key9 => '9',
            Key10 => 'A',
            Key11 => 'B',
            Key12 => 'C',
            Key13 => 'D',
            Key14 => 'E',
            Scratch => '6',
            ScratchExtra => '7',
            FootPedal => 'F',
            FreeZone => '7',
        };
        NoteChannel::try_from_chars(side_char, key_char).expect("valid base62 for BeatKey")
    }

    fn from_note_channel(channel: NoteChannel) -> Option<Self> {
        // Restore side and key from two characters:
        let [c1, c2] = channel.to_str();
        // Side: read according to general rules (1/3/5/D -> P1, 2/4/6/E -> P2)
        let side = match c1.to_ascii_uppercase() {
            '1' | '3' | '5' | 'D' => PlayerSide::Player1,
            '2' | '4' | '6' | 'E' => PlayerSide::Player2,
            _ => return None,
        };
        use Key::*;
        let key = match c2.to_ascii_uppercase() {
            '1' => Key1,
            '2' => Key2,
            '3' => Key3,
            '4' => Key4,
            '5' => Key5,
            '6' => Scratch,
            '7' => FreeZone, // Or ScratchExtra: here '7' is treated as FreeZone according to general character table, mapped in specific mode later
            '8' => Key6,
            '9' => Key7,
            'A' => Key10,
            'B' => Key11,
            'C' => Key12,
            'D' => Key13,
            'E' => Key14,
            'F' => FootPedal,
            _ => return None,
        };
        let kind = NoteKind::note_kind_from_channel(channel)?;
        Some(Self { side, key, kind })
    }
}

/// PMS BME-type physical key (supports 9K/18K), mapping via KeyLayoutPmsBmeType
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PmsBmeKey {
    /// The side of the player.
    pub side: PlayerSide,
    /// The key of the player.
    pub key: Key,
    /// The kind of note.
    pub kind: NoteKind,
}

impl PmsBmeKey {
    /// Create a new [`PmsBmeKey`] from a [`PlayerSide`], [`Key`] and [`NoteKind`].
    pub const fn new(side: PlayerSide, key: Key, kind: NoteKind) -> Self {
        Self { side, key, kind }
    }

    /// Create a new [`PmsBmeKey`] from a [`PlayerSide`] and [`Key`] with default [`NoteKind`].
    pub const fn new_with_default_kind(side: PlayerSide, key: Key) -> Self {
        Self::new(side, key, NoteKind::Visible)
    }
}

impl KeyMapping for PmsBmeKey {
    fn new(side: PlayerSide, key: Key, kind: NoteKind) -> Self {
        Self::new(side, key, kind)
    }
    fn side(&self) -> PlayerSide {
        self.side
    }
    fn key(&self) -> Key {
        self.key
    }
    fn kind(&self) -> NoteKind {
        self.kind
    }
    fn into_tuple(self) -> (PlayerSide, Key, NoteKind) {
        (self.side, self.key, self.kind)
    }
}

impl PhysicalKey for PmsBmeKey {
    fn to_note_channel(self) -> NoteChannel {
        use Key::*;
        let mapped_key = match self.key {
            Key8 => Scratch,
            Key9 => FreeZone,
            other => other,
        };
        BeatKey::new(self.side, mapped_key, self.kind).to_note_channel()
    }

    fn from_note_channel(channel: NoteChannel) -> Option<Self> {
        use Key::*;
        let beat = BeatKey::from_note_channel(channel)?;
        let key = match beat.key {
            Scratch => Key8,
            FreeZone => Key9,
            other => other,
        };
        Some(Self {
            side: beat.side,
            key,
            kind: beat.kind,
        })
    }
}

/// PMS physical key, mapping via KeyLayoutPms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PmsKey {
    /// The side of the player.
    pub side: PlayerSide,
    /// The key of the player.
    pub key: Key,
    /// The kind of note.
    pub kind: NoteKind,
}

impl PmsKey {
    /// Create a new [`PmsKey`] from a [`PlayerSide`], [`Key`] and [`NoteKind`].
    pub const fn new(side: PlayerSide, key: Key, kind: NoteKind) -> Self {
        Self { side, key, kind }
    }

    /// Create a new [`PmsKey`] from a [`PlayerSide`] and [`Key`] with default [`NoteKind`].
    pub const fn new_with_default_kind(side: PlayerSide, key: Key) -> Self {
        Self::new(side, key, NoteKind::Visible)
    }
}

impl KeyMapping for PmsKey {
    fn new(side: PlayerSide, key: Key, kind: NoteKind) -> Self {
        Self::new(side, key, kind)
    }
    fn side(&self) -> PlayerSide {
        self.side
    }
    fn key(&self) -> Key {
        self.key
    }
    fn kind(&self) -> NoteKind {
        self.kind
    }
    fn into_tuple(self) -> (PlayerSide, Key, NoteKind) {
        (self.side, self.key, self.kind)
    }
}
impl PhysicalKey for PmsKey {
    fn to_note_channel(self) -> NoteChannel {
        use Key::*;
        use PlayerSide::*;
        let (side, key) = match (self.side, self.key) {
            (Player1, Key6) => (Player2, Key2),
            (Player1, Key7) => (Player2, Key3),
            (Player1, Key8) => (Player2, Key4),
            (Player1, Key9) => (Player2, Key5),
            other => other,
        };
        BeatKey::new(side, key, self.kind).to_note_channel()
    }

    fn from_note_channel(channel: NoteChannel) -> Option<Self> {
        use Key::*;
        use PlayerSide::*;
        let beat = BeatKey::from_note_channel(channel)?;
        let (side, key) = match (beat.side, beat.key) {
            (Player1, Key1 | Key2 | Key3 | Key4 | Key5) => (Player1, beat.key),
            (Player2, Key2) => (Player1, Key6),
            (Player2, Key3) => (Player1, Key7),
            (Player2, Key4) => (Player1, Key8),
            (Player2, Key5) => (Player1, Key9),
            other => other,
        };
        Some(Self {
            side,
            key,
            kind: beat.kind,
        })
    }
}

/// Beat nanasi/angolmois physical key, mapping via KeyLayoutBeatNanasi
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BeatNanasiKey {
    /// The side of the player.
    pub side: PlayerSide,
    /// The key of the player.
    pub key: Key,
    /// The kind of note.
    pub kind: NoteKind,
}

impl BeatNanasiKey {
    /// Create a new [`BeatNanasiKey`] from a [`PlayerSide`], [`Key`] and [`NoteKind`].
    pub const fn new(side: PlayerSide, key: Key, kind: NoteKind) -> Self {
        Self { side, key, kind }
    }

    /// Create a new [`BeatNanasiKey`] from a [`PlayerSide`] and [`Key`] with default [`NoteKind`].
    pub const fn new_with_default_kind(side: PlayerSide, key: Key) -> Self {
        Self::new(side, key, NoteKind::Visible)
    }
}

impl KeyMapping for BeatNanasiKey {
    fn new(side: PlayerSide, key: Key, kind: NoteKind) -> Self {
        Self::new(side, key, kind)
    }
    fn side(&self) -> PlayerSide {
        self.side
    }
    fn key(&self) -> Key {
        self.key
    }
    fn kind(&self) -> NoteKind {
        self.kind
    }
    fn into_tuple(self) -> (PlayerSide, Key, NoteKind) {
        (self.side, self.key, self.kind)
    }
}

impl PhysicalKey for BeatNanasiKey {
    fn to_note_channel(self) -> NoteChannel {
        use Key::*;
        let key = match self.key {
            FootPedal => FreeZone,
            other => other,
        };
        BeatKey::new(self.side, key, self.kind).to_note_channel()
    }

    fn from_note_channel(channel: NoteChannel) -> Option<Self> {
        use Key::*;
        let beat = BeatKey::from_note_channel(channel)?;
        let key = match beat.key {
            FreeZone => FootPedal,
            other => other,
        };
        Some(Self {
            side: beat.side,
            key,
            kind: beat.kind,
        })
    }
}

/// DSC & OCT/FP physical key, mapping via KeyLayoutDscOctFp
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DscOctFpKey {
    /// The side of the player.
    pub side: PlayerSide,
    /// The key of the player.
    pub key: Key,
    /// The kind of note.
    pub kind: NoteKind,
}

impl DscOctFpKey {
    /// Create a new [`DscOctFpKey`] from a [`PlayerSide`], [`Key`] and [`NoteKind`].
    pub const fn new(side: PlayerSide, key: Key, kind: NoteKind) -> Self {
        Self { side, key, kind }
    }

    /// Create a new [`DscOctFpKey`] from a [`PlayerSide`] and [`Key`] with default [`NoteKind`].
    pub const fn new_with_default_kind(side: PlayerSide, key: Key) -> Self {
        Self::new(side, key, NoteKind::Visible)
    }
}

impl KeyMapping for DscOctFpKey {
    fn new(side: PlayerSide, key: Key, kind: NoteKind) -> Self {
        Self::new(side, key, kind)
    }
    fn side(&self) -> PlayerSide {
        self.side
    }
    fn key(&self) -> Key {
        self.key
    }
    fn kind(&self) -> NoteKind {
        self.kind
    }
    fn into_tuple(self) -> (PlayerSide, Key, NoteKind) {
        (self.side, self.key, self.kind)
    }
}

impl PhysicalKey for DscOctFpKey {
    fn to_note_channel(self) -> NoteChannel {
        use Key::*;
        use PlayerSide::*;
        let (side, key) = match (self.side, self.key) {
            (Player1, ScratchExtra) => (Player2, Scratch),
            (Player1, FootPedal) => (Player2, Key1),
            (Player1, Key8) => (Player2, Key2),
            (Player1, Key9) => (Player2, Key3),
            (Player1, Key10) => (Player2, Key4),
            (Player1, Key11) => (Player2, Key5),
            (Player1, Key12) => (Player2, Key6),
            (Player1, Key13) => (Player2, Key7),
            (s, k) => (s, k),
        };
        BeatKey::new(side, key, self.kind).to_note_channel()
    }

    fn from_note_channel(channel: NoteChannel) -> Option<Self> {
        use Key::*;
        use PlayerSide::*;
        let beat = BeatKey::from_note_channel(channel)?;
        let (side, key) = match (beat.side, beat.key) {
            (Player2, Key1) => (Player1, FootPedal),
            (Player2, Key2) => (Player1, Key8),
            (Player2, Key3) => (Player1, Key9),
            (Player2, Key4) => (Player1, Key10),
            (Player2, Key5) => (Player1, Key11),
            (Player2, Key6) => (Player1, Key12),
            (Player2, Key7) => (Player1, Key13),
            (Player2, Scratch) => (Player1, ScratchExtra),
            (s, k) => (s, k),
        };
        Some(Self {
            side,
            key,
            kind: beat.kind,
        })
    }
}

/// A generic 1P-only N-keys physical layout. Keys are mapped to BeatKey Player1 Key1..KeyN.
/// This implements PhysicalKey but intentionally does NOT implement KeyMapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenericNKey<const N: usize> {
    /// 1-based key index within [1, N]
    pub index: usize,
}

impl<const N: usize> GenericNKey<N> {
    /// Create a new [`GenericNKey`] from an index within `[1, N]`.
    pub const fn new(index: usize) -> Self {
        Self { index }
    }
}

impl<const N: usize> PhysicalKey for GenericNKey<N> {
    fn to_note_channel(self) -> NoteChannel {
        use super::Key::*;
        let key = match self.index {
            1 if N >= 1 => Key1,
            2 if N >= 2 => Key2,
            3 if N >= 3 => Key3,
            4 if N >= 4 => Key4,
            5 if N >= 5 => Key5,
            6 if N >= 6 => Key6,
            7 if N >= 7 => Key7,
            8 if N >= 8 => Key8,
            9 if N >= 9 => Key9,
            10 if N >= 10 => Key10,
            11 if N >= 11 => Key11,
            12 if N >= 12 => Key12,
            13 if N >= 13 => Key13,
            14 if N >= 14 => Key14,
            // Fallback to Key1 if out of range; this should be validated by constructors in usage
            _ => Key1,
        };
        BeatKey::new(PlayerSide::Player1, key, NoteKind::Visible).to_note_channel()
    }

    fn from_note_channel(channel: NoteChannel) -> Option<Self> {
        use super::Key::*;
        let beat = BeatKey::from_note_channel(channel)?;
        if beat.side != PlayerSide::Player1 {
            return None;
        }
        let index = match beat.key {
            Key1 => 1,
            Key2 => 2,
            Key3 => 3,
            Key4 => 4,
            Key5 => 5,
            Key6 => 6,
            Key7 => 7,
            Key8 => 8,
            Key9 => 9,
            Key10 => 10,
            Key11 => 11,
            Key12 => 12,
            Key13 => 13,
            Key14 => 14,
            _ => return None, // excludes Scratch/FootPedal/FreeZone/etc.
        };
        if index == 0 || index > N {
            return None;
        }
        Some(Self { index })
    }
}

#[cfg(test)]
mod layout_roundtrip_tests {
    use super::*;

    fn roundtrip<T: PhysicalKey>(side: PlayerSide, key: Key) -> Option<(PlayerSide, Key)> {
        let chan = BeatKey::new(side, key, NoteKind::Visible).to_note_channel();
        // map: Beat -> Target -> Beat
        let target = T::from_note_channel(chan)?;
        let back = BeatKey::from_note_channel(target.to_note_channel())?;
        Some((back.side, back.key))
    }

    #[test]
    fn pms_bme_roundtrip() {
        use PlayerSide::*;
        for &(s, k) in &[
            (Player1, Key::Key1),
            (Player1, Key::Key2),
            (Player1, Key::Key3),
            (Player1, Key::Key4),
            (Player1, Key::Key5),
            (Player1, Key::Key6),
            (Player1, Key::Key7),
            (Player1, Key::Scratch),
            (Player1, Key::FreeZone),
            (Player2, Key::Key1),
            (Player2, Key::Key2),
            (Player2, Key::Key3),
            (Player2, Key::Key4),
            (Player2, Key::Key5),
            (Player2, Key::Key6),
            (Player2, Key::Key7),
            (Player2, Key::Scratch),
            (Player2, Key::FreeZone),
        ] {
            let got = roundtrip::<PmsBmeKey>(s, k).unwrap();
            assert_eq!(got, (s, k));
        }
    }

    #[test]
    fn pms_roundtrip() {
        use PlayerSide::*;
        // Only test inputs within the canonical Beat domain for PMS mapping
        for &(s, k) in &[
            (Player1, Key::Key1),
            (Player1, Key::Key2),
            (Player1, Key::Key3),
            (Player1, Key::Key4),
            (Player1, Key::Key5),
            (Player2, Key::Key2),
            (Player2, Key::Key3),
            (Player2, Key::Key4),
            (Player2, Key::Key5),
        ] {
            let got = roundtrip::<PmsKey>(s, k).unwrap();
            assert_eq!(got, (s, k));
        }
    }

    #[test]
    fn nanasi_roundtrip() {
        use PlayerSide::*;
        for &(s, k) in &[
            (Player1, Key::Key1),
            (Player1, Key::Key2),
            (Player1, Key::Key3),
            (Player1, Key::Key4),
            (Player1, Key::Key5),
            (Player1, Key::Key6),
            (Player1, Key::Key7),
            (Player1, Key::Scratch),
            (Player1, Key::FreeZone),
            (Player2, Key::Key1),
            (Player2, Key::Key2),
            (Player2, Key::Key3),
            (Player2, Key::Key4),
            (Player2, Key::Key5),
            (Player2, Key::Key6),
            (Player2, Key::Key7),
            (Player2, Key::Scratch),
            (Player2, Key::FreeZone),
        ] {
            let got = roundtrip::<BeatNanasiKey>(s, k).unwrap();
            assert_eq!(got, (s, k));
        }
    }

    #[test]
    fn dsc_octfp_roundtrip() {
        use PlayerSide::*;
        // Only test inputs within the canonical Beat domain for DSC/OCT-FP mapping
        for &(s, k) in &[
            (Player1, Key::Key1),
            (Player1, Key::Key2),
            (Player1, Key::Key3),
            (Player1, Key::Key4),
            (Player1, Key::Key5),
            (Player1, Key::Key6),
            (Player1, Key::Key7),
            (Player1, Key::Scratch),
            (Player2, Key::Key1),
            (Player2, Key::Key2),
            (Player2, Key::Key3),
            (Player2, Key::Key4),
            (Player2, Key::Key5),
            (Player2, Key::Key6),
            (Player2, Key::Key7),
            (Player2, Key::Scratch),
        ] {
            let got = roundtrip::<DscOctFpKey>(s, k).unwrap();
            assert_eq!(got, (s, k));
        }
    }
}
