//! Mappers between modes

use super::{Key, NoteChannel, NoteKind, PlayerSide};

/// Maximum key index for regular keys (1-14)
pub const MAX_KEY_INDEX: u8 = 14;
/// Maximum scratch index (1-255, but typically 1-2)
pub const MAX_SCRATCH_INDEX: u8 = 2;
/// Maximum scratch extra index (1-255, but typically 1)
pub const MAX_SCRATCH_EXTRA_INDEX: u8 = 1;

/// A trait that maps between logical [`NoteChannel`] and concrete physical key layouts.
/// This trait combines key mapping storage with physical key conversion functionality.
pub trait KeyMapping: Copy + Eq + core::fmt::Debug {
    /// Create a new [`KeyMapping`] from a [`PlayerSide`], [`Key`] and [`NoteKind`].
    fn new(side: PlayerSide, key: Key, kind: NoteKind) -> Self;
    /// Get the PlayerSide from this KeyMapping.
    fn side(&self) -> PlayerSide;
    /// Get the [`Key`] from this [`KeyMapping`].
    fn key(&self) -> Key;
    /// Get the [`NoteKind`] from this [`KeyMapping`].
    fn kind(&self) -> NoteKind;
    /// Deconstruct into a [`PlayerSide`], [`Key`], [`NoteKind`] tuple.
    fn into_tuple(self) -> (PlayerSide, Key, NoteKind);
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
}

impl Default for BeatKey {
    fn default() -> Self {
        Self {
            side: PlayerSide::default(),
            key: super::Key::new_key(1).unwrap(),
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

    fn to_note_channel(self) -> NoteChannel {
        // Map side and key to base62 two-character:
        // First character: side (using visible note default: P1='1', P2='2')
        // Second character: key code ('1'..'5', '6'=Scratch, '7'=FreeZone, '8'=Key6/'Key8, '9'=Key7/'Key9,
        //                      'A'..'E'=Key10..Key14, 'F'=FootPedal, '7'=ScratchExtra)
        let side_char = match self.side {
            PlayerSide::Player1 => '1',
            PlayerSide::Player2 => '2',
        };
        let key_char = match self.key {
            Key::Key(idx) => match idx.get() {
                1 => '1',
                2 => '2',
                3 => '3',
                4 => '4',
                5 => '5',
                6 => '8',
                7 => '9',
                8 => '8',
                9 => '9',
                10 => 'A',
                11 => 'B',
                12 => 'C',
                13 => 'D',
                14 => 'E',
                _ => unreachable!("Key index should be 1-14"),
            },
            Key::Scratch(_) => '6',
            Key::ScratchExtra(_) => '7',
            Key::FootPedal => 'F',
            Key::FreeZone => '7',
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
        let key = match c2.to_ascii_uppercase() {
            '1' => Key::new_key(1).unwrap(),
            '2' => Key::new_key(2).unwrap(),
            '3' => Key::new_key(3).unwrap(),
            '4' => Key::new_key(4).unwrap(),
            '5' => Key::new_key(5).unwrap(),
            '6' => Key::new_scratch(1).unwrap(), // Default scratch index
            '7' => super::Key::FreeZone, // Or ScratchExtra: here '7' is treated as FreeZone according to general character table, mapped in specific mode later
            '8' => Key::new_key(6).unwrap(),
            '9' => Key::new_key(7).unwrap(),
            'A' => Key::new_key(10).unwrap(),
            'B' => Key::new_key(11).unwrap(),
            'C' => Key::new_key(12).unwrap(),
            'D' => Key::new_key(13).unwrap(),
            'E' => Key::new_key(14).unwrap(),
            'F' => Key::FootPedal,
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

    fn to_note_channel(self) -> NoteChannel {
        let mapped_key = match self.key {
            Key::Key(idx) if idx.get() == 8 => Key::new_scratch(1).unwrap(),
            Key::Key(idx) if idx.get() == 9 => super::Key::FreeZone,
            other => other,
        };
        BeatKey::new(self.side, mapped_key, self.kind).to_note_channel()
    }

    fn from_note_channel(channel: NoteChannel) -> Option<Self> {
        let beat = BeatKey::from_note_channel(channel)?;
        let key = match beat.key {
            Key::Scratch(idx) if idx.get() == 1 => Key::new_key(8).unwrap(),
            Key::FreeZone => Key::new_key(9).unwrap(),
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

    fn to_note_channel(self) -> NoteChannel {
        use PlayerSide::*;
        let (side, key) = match (self.side, self.key) {
            (Player1, Key::Key(idx)) if idx.get() == 6 => (Player2, Key::new_key(2).unwrap()),
            (Player1, Key::Key(idx)) if idx.get() == 7 => (Player2, Key::new_key(3).unwrap()),
            (Player1, Key::Key(idx)) if idx.get() == 8 => (Player2, Key::new_key(4).unwrap()),
            (Player1, Key::Key(idx)) if idx.get() == 9 => (Player2, Key::new_key(5).unwrap()),
            _ => (self.side, self.key),
        };
        BeatKey::new(side, key, self.kind).to_note_channel()
    }

    fn from_note_channel(channel: NoteChannel) -> Option<Self> {
        use PlayerSide::*;
        let beat = BeatKey::from_note_channel(channel)?;
        let (side, key) = match (beat.side, beat.key) {
            (Player1, key) if matches!(key, Key::Key(idx) if idx.get() >= 1 && idx.get() <= 5) => {
                (Player1, key)
            }
            (Player2, Key::Key(idx)) if idx.get() == 2 => (Player1, Key::new_key(6).unwrap()),
            (Player2, Key::Key(idx)) if idx.get() == 3 => (Player1, Key::new_key(7).unwrap()),
            (Player2, Key::Key(idx)) if idx.get() == 4 => (Player1, Key::new_key(8).unwrap()),
            (Player2, Key::Key(idx)) if idx.get() == 5 => (Player1, Key::new_key(9).unwrap()),
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

    fn to_note_channel(self) -> NoteChannel {
        let key = match self.key {
            Key::FootPedal => super::Key::FreeZone,
            other => other,
        };
        BeatKey::new(self.side, key, self.kind).to_note_channel()
    }

    fn from_note_channel(channel: NoteChannel) -> Option<Self> {
        let beat = BeatKey::from_note_channel(channel)?;
        let key = match beat.key {
            super::Key::FreeZone => Key::FootPedal,
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

    fn to_note_channel(self) -> NoteChannel {
        use PlayerSide::*;
        let (side, key) = match (self.side, self.key) {
            (Player1, Key::ScratchExtra(idx)) if idx.get() == 1 => {
                (Player2, Key::new_scratch(1).unwrap())
            }
            (Player1, Key::FootPedal) => (Player2, Key::new_key(1).unwrap()),
            (Player1, Key::Key(idx)) if idx.get() == 8 => (Player2, Key::new_key(2).unwrap()),
            (Player1, Key::Key(idx)) if idx.get() == 9 => (Player2, Key::new_key(3).unwrap()),
            (Player1, Key::Key(idx)) if idx.get() == 10 => (Player2, Key::new_key(4).unwrap()),
            (Player1, Key::Key(idx)) if idx.get() == 11 => (Player2, Key::new_key(5).unwrap()),
            (Player1, Key::Key(idx)) if idx.get() == 12 => (Player2, Key::new_key(6).unwrap()),
            (Player1, Key::Key(idx)) if idx.get() == 13 => (Player2, Key::new_key(7).unwrap()),
            (s, k) => (s, k),
        };
        BeatKey::new(side, key, self.kind).to_note_channel()
    }

    fn from_note_channel(channel: NoteChannel) -> Option<Self> {
        use PlayerSide::*;
        let beat = BeatKey::from_note_channel(channel)?;
        let (side, key) = match (beat.side, beat.key) {
            (Player2, Key::Key(idx)) if idx.get() == 1 => (Player1, Key::FootPedal),
            (Player2, Key::Key(idx)) if idx.get() == 2 => (Player1, Key::new_key(8).unwrap()),
            (Player2, Key::Key(idx)) if idx.get() == 3 => (Player1, Key::new_key(9).unwrap()),
            (Player2, Key::Key(idx)) if idx.get() == 4 => (Player1, Key::new_key(10).unwrap()),
            (Player2, Key::Key(idx)) if idx.get() == 5 => (Player1, Key::new_key(11).unwrap()),
            (Player2, Key::Key(idx)) if idx.get() == 6 => (Player1, Key::new_key(12).unwrap()),
            (Player2, Key::Key(idx)) if idx.get() == 7 => (Player1, Key::new_key(13).unwrap()),
            (Player2, Key::Scratch(idx)) if idx.get() == 1 => {
                (Player1, Key::new_scratch_extra(1).unwrap())
            }
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

impl<const N: usize> KeyMapping for GenericNKey<N> {
    fn new(_side: PlayerSide, _key: Key, _kind: NoteKind) -> Self {
        // GenericNKey doesn't store side/key/kind, always create with default
        Self::new(1)
    }

    fn side(&self) -> PlayerSide {
        PlayerSide::Player1 // Always Player1 for GenericNKey
    }

    fn key(&self) -> super::Key {
        // Convert index to Key
        match self.index {
            1 if N >= 1 => super::Key::new_key(1).unwrap(),
            2 if N >= 2 => super::Key::new_key(2).unwrap(),
            3 if N >= 3 => super::Key::new_key(3).unwrap(),
            4 if N >= 4 => super::Key::new_key(4).unwrap(),
            5 if N >= 5 => super::Key::new_key(5).unwrap(),
            6 if N >= 6 => super::Key::new_key(6).unwrap(),
            7 if N >= 7 => super::Key::new_key(7).unwrap(),
            8 if N >= 8 => super::Key::new_key(8).unwrap(),
            9 if N >= 9 => super::Key::new_key(9).unwrap(),
            10 if N >= 10 => super::Key::new_key(10).unwrap(),
            11 if N >= 11 => super::Key::new_key(11).unwrap(),
            12 if N >= 12 => super::Key::new_key(12).unwrap(),
            13 if N >= 13 => super::Key::new_key(13).unwrap(),
            14 if N >= 14 => super::Key::new_key(14).unwrap(),
            _ => super::Key::new_key(1).unwrap(), // fallback
        }
    }

    fn kind(&self) -> NoteKind {
        NoteKind::Visible // Always Visible for GenericNKey
    }

    fn into_tuple(self) -> (PlayerSide, super::Key, NoteKind) {
        (self.side(), self.key(), self.kind())
    }

    fn to_note_channel(self) -> NoteChannel {
        let key = match self.index {
            1 if N >= 1 => super::Key::new_key(1).unwrap(),
            2 if N >= 2 => super::Key::new_key(2).unwrap(),
            3 if N >= 3 => super::Key::new_key(3).unwrap(),
            4 if N >= 4 => super::Key::new_key(4).unwrap(),
            5 if N >= 5 => super::Key::new_key(5).unwrap(),
            6 if N >= 6 => super::Key::new_key(6).unwrap(),
            7 if N >= 7 => super::Key::new_key(7).unwrap(),
            8 if N >= 8 => super::Key::new_key(8).unwrap(),
            9 if N >= 9 => super::Key::new_key(9).unwrap(),
            10 if N >= 10 => super::Key::new_key(10).unwrap(),
            11 if N >= 11 => super::Key::new_key(11).unwrap(),
            12 if N >= 12 => super::Key::new_key(12).unwrap(),
            13 if N >= 13 => super::Key::new_key(13).unwrap(),
            14 if N >= 14 => super::Key::new_key(14).unwrap(),
            // Fallback to Key1 if out of range; this should be validated by constructors in usage
            _ => super::Key::new_key(1).unwrap(),
        };
        BeatKey::new(PlayerSide::Player1, key, NoteKind::Visible).to_note_channel()
    }

    fn from_note_channel(channel: NoteChannel) -> Option<Self> {
        let beat = BeatKey::from_note_channel(channel)?;
        if beat.side != PlayerSide::Player1 {
            return None;
        }
        let index = match beat.key {
            super::Key::Key(idx) => idx.get() as usize,
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

    fn roundtrip<T: KeyMapping>(
        side: PlayerSide,
        key: super::Key,
    ) -> Option<(PlayerSide, super::Key)> {
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
            (Player1, super::Key::new_key(1).unwrap()),
            (Player1, super::Key::new_key(2).unwrap()),
            (Player1, super::Key::new_key(3).unwrap()),
            (Player1, super::Key::new_key(4).unwrap()),
            (Player1, super::Key::new_key(5).unwrap()),
            (Player1, super::Key::new_key(6).unwrap()),
            (Player1, super::Key::new_key(7).unwrap()),
            (Player1, super::Key::new_scratch(1).unwrap()),
            (Player1, super::Key::FreeZone),
            (Player2, super::Key::new_key(1).unwrap()),
            (Player2, super::Key::new_key(2).unwrap()),
            (Player2, super::Key::new_key(3).unwrap()),
            (Player2, super::Key::new_key(4).unwrap()),
            (Player2, super::Key::new_key(5).unwrap()),
            (Player2, super::Key::new_key(6).unwrap()),
            (Player2, super::Key::new_key(7).unwrap()),
            (Player2, super::Key::new_scratch(1).unwrap()),
            (Player2, super::Key::FreeZone),
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
            (Player1, super::Key::new_key(1).unwrap()),
            (Player1, super::Key::new_key(2).unwrap()),
            (Player1, super::Key::new_key(3).unwrap()),
            (Player1, super::Key::new_key(4).unwrap()),
            (Player1, super::Key::new_key(5).unwrap()),
            (Player2, super::Key::new_key(2).unwrap()),
            (Player2, super::Key::new_key(3).unwrap()),
            (Player2, super::Key::new_key(4).unwrap()),
            (Player2, super::Key::new_key(5).unwrap()),
        ] {
            let got = roundtrip::<PmsKey>(s, k).unwrap();
            assert_eq!(got, (s, k));
        }
    }

    #[test]
    fn nanasi_roundtrip() {
        use PlayerSide::*;
        for &(s, k) in &[
            (Player1, super::Key::new_key(1).unwrap()),
            (Player1, super::Key::new_key(2).unwrap()),
            (Player1, super::Key::new_key(3).unwrap()),
            (Player1, super::Key::new_key(4).unwrap()),
            (Player1, super::Key::new_key(5).unwrap()),
            (Player1, super::Key::new_key(6).unwrap()),
            (Player1, super::Key::new_key(7).unwrap()),
            (Player1, super::Key::new_scratch(1).unwrap()),
            (Player1, super::Key::FreeZone),
            (Player2, super::Key::new_key(1).unwrap()),
            (Player2, super::Key::new_key(2).unwrap()),
            (Player2, super::Key::new_key(3).unwrap()),
            (Player2, super::Key::new_key(4).unwrap()),
            (Player2, super::Key::new_key(5).unwrap()),
            (Player2, super::Key::new_key(6).unwrap()),
            (Player2, super::Key::new_key(7).unwrap()),
            (Player2, super::Key::new_scratch(1).unwrap()),
            (Player2, super::Key::FreeZone),
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
            (Player1, super::Key::new_key(1).unwrap()),
            (Player1, super::Key::new_key(2).unwrap()),
            (Player1, super::Key::new_key(3).unwrap()),
            (Player1, super::Key::new_key(4).unwrap()),
            (Player1, super::Key::new_key(5).unwrap()),
            (Player1, super::Key::new_key(6).unwrap()),
            (Player1, super::Key::new_key(7).unwrap()),
            (Player1, super::Key::new_scratch(1).unwrap()),
            (Player2, super::Key::new_key(1).unwrap()),
            (Player2, super::Key::new_key(2).unwrap()),
            (Player2, super::Key::new_key(3).unwrap()),
            (Player2, super::Key::new_key(4).unwrap()),
            (Player2, super::Key::new_key(5).unwrap()),
            (Player2, super::Key::new_key(6).unwrap()),
            (Player2, super::Key::new_key(7).unwrap()),
            (Player2, super::Key::new_scratch(1).unwrap()),
        ] {
            let got = roundtrip::<DscOctFpKey>(s, k).unwrap();
            assert_eq!(got, (s, k));
        }
    }
}
