//! Mappers between modes

use super::{Key, NoteChannel, PlayerSide};

/// A trait for key mapping storage structure.
pub trait KeyMapping {
    /// Create a new [`KeyMapping`] from a [`PlayerSide`] and [`Key`].
    fn new(side: PlayerSide, key: Key) -> Self;
    /// Get the PlayerSide from this KeyMapping.
    fn side(&self) -> PlayerSide;
    /// Get the [`Key`] from this [`KeyMapping`].
    fn key(&self) -> Key;
    /// Deconstruct into a [`PlayerSide`], [`Key`] tuple.
    fn into_tuple(self) -> (PlayerSide, Key);
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
}

impl BeatKey {
    /// Create a new [`BeatKey`] from a [`PlayerSide`] and [`Key`].
    pub const fn new(side: PlayerSide, key: Key) -> Self {
        Self { side, key }
    }
}

impl Default for BeatKey {
    fn default() -> Self {
        Self {
            side: PlayerSide::default(),
            key: Key::Key1,
        }
    }
}

impl KeyMapping for BeatKey {
    fn new(side: PlayerSide, key: Key) -> Self {
        Self::new(side, key)
    }
    fn side(&self) -> PlayerSide {
        self.side
    }
    fn key(&self) -> Key {
        self.key
    }
    fn into_tuple(self) -> (PlayerSide, Key) {
        (self.side, self.key)
    }
}

impl PhysicalKey for BeatKey {
    fn to_note_channel(self) -> NoteChannel {
        // Encode side and key into a stable lane id space.
        // Player1 base 0, Player2 base 1000.
        let base = match self.side {
            PlayerSide::Player1 => 0u32,
            PlayerSide::Player2 => 1000u32,
        };
        use Key::*;
        let local = match self.key {
            Key1 => 0,
            Key2 => 1,
            Key3 => 2,
            Key4 => 3,
            Key5 => 4,
            Key6 => 5,
            Key7 => 6,
            Key8 => 7,
            Key9 => 8,
            Key10 => 9,
            Key11 => 10,
            Key12 => 11,
            Key13 => 12,
            Key14 => 13,
            Scratch => 100,
            ScratchExtra => 101,
            FootPedal => 150,
            FreeZone => 200,
        } as u32;
        NoteChannel(base + local)
    }

    fn from_note_channel(channel: NoteChannel) -> Option<Self> {
        let v = channel.0;
        let (side, local) = if v >= 1000 {
            (PlayerSide::Player2, v - 1000)
        } else {
            (PlayerSide::Player1, v)
        };
        use Key::*;
        let key = match local {
            0 => Key1,
            1 => Key2,
            2 => Key3,
            3 => Key4,
            4 => Key5,
            5 => Key6,
            6 => Key7,
            7 => Key8,
            8 => Key9,
            9 => Key10,
            10 => Key11,
            11 => Key12,
            12 => Key13,
            13 => Key14,
            100 => Scratch,
            101 => ScratchExtra,
            150 => FootPedal,
            200 => FreeZone,
            _ => return None,
        };
        Some(Self { side, key })
    }
}

/// PMS BME-type physical key (supports 9K/18K), mapping via KeyLayoutPmsBmeType
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PmsBmeKey {
    /// The side of the player.
    pub side: PlayerSide,
    /// The key of the player.
    pub key: Key,
}

impl PmsBmeKey {
    /// Create a new [`PmsBmeKey`] from a [`PlayerSide`] and [`Key`].
    pub const fn new(side: PlayerSide, key: Key) -> Self {
        Self { side, key }
    }
}

impl KeyMapping for PmsBmeKey {
    fn new(side: PlayerSide, key: Key) -> Self {
        Self::new(side, key)
    }
    fn side(&self) -> PlayerSide {
        self.side
    }
    fn key(&self) -> Key {
        self.key
    }
    fn into_tuple(self) -> (PlayerSide, Key) {
        (self.side, self.key)
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
        BeatKey::new(self.side, mapped_key).to_note_channel()
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
}

impl PmsKey {
    /// Create a new [`PmsKey`] from a [`PlayerSide`] and [`Key`].
    pub const fn new(side: PlayerSide, key: Key) -> Self {
        Self { side, key }
    }
}

impl KeyMapping for PmsKey {
    fn new(side: PlayerSide, key: Key) -> Self {
        Self::new(side, key)
    }
    fn side(&self) -> PlayerSide {
        self.side
    }
    fn key(&self) -> Key {
        self.key
    }
    fn into_tuple(self) -> (PlayerSide, Key) {
        (self.side, self.key)
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
        BeatKey::new(side, key).to_note_channel()
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
        Some(Self { side, key })
    }
}

/// Beat nanasi/angolmois physical key, mapping via KeyLayoutBeatNanasi
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BeatNanasiKey {
    /// The side of the player.
    pub side: PlayerSide,
    /// The key of the player.
    pub key: Key,
}

impl BeatNanasiKey {
    /// Create a new [`BeatNanasiKey`] from a [`PlayerSide`] and [`Key`].
    pub const fn new(side: PlayerSide, key: Key) -> Self {
        Self { side, key }
    }
}

impl KeyMapping for BeatNanasiKey {
    fn new(side: PlayerSide, key: Key) -> Self {
        Self::new(side, key)
    }
    fn side(&self) -> PlayerSide {
        self.side
    }
    fn key(&self) -> Key {
        self.key
    }
    fn into_tuple(self) -> (PlayerSide, Key) {
        (self.side, self.key)
    }
}

impl PhysicalKey for BeatNanasiKey {
    fn to_note_channel(self) -> NoteChannel {
        use Key::*;
        let key = match self.key {
            FootPedal => FreeZone,
            other => other,
        };
        BeatKey::new(self.side, key).to_note_channel()
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
}

impl DscOctFpKey {
    /// Create a new [`DscOctFpKey`] from a [`PlayerSide`] and [`Key`].
    pub const fn new(side: PlayerSide, key: Key) -> Self {
        Self { side, key }
    }
}

impl KeyMapping for DscOctFpKey {
    fn new(side: PlayerSide, key: Key) -> Self {
        Self::new(side, key)
    }
    fn side(&self) -> PlayerSide {
        self.side
    }
    fn key(&self) -> Key {
        self.key
    }
    fn into_tuple(self) -> (PlayerSide, Key) {
        (self.side, self.key)
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
        BeatKey::new(side, key).to_note_channel()
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
        Some(Self { side, key })
    }
}

#[cfg(test)]
mod layout_roundtrip_tests {
    use super::*;

    fn roundtrip<T: PhysicalKey>(side: PlayerSide, key: Key) -> Option<(PlayerSide, Key)> {
        let chan = BeatKey::new(side, key).to_note_channel();
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
