//! For converting key/channel between different modes, please see [`KeyLayoutMapper`] enum and [`convert_key_mapping_between`] function.

use super::{Key, NoteKind, PlayerSide};
use Key::*;

/// A trait for key mapping storage structure.
pub trait KeyMapping {
    /// Create a new [`KeyMapping`] from a [`PlayerSide`], [`NoteKind`] and [`Key`].
    fn new(side: PlayerSide, kind: NoteKind, key: Key) -> Self;
    /// Get the PlayerSide from this KeyMapping.
    fn side(&self) -> PlayerSide;
    /// Get the NoteKind from this KeyMapping.
    fn kind(&self) -> NoteKind;
    /// Get the [`Key`] from this [`KeyMapping`].
    fn key(&self) -> Key;
    /// Deconstruct into a [`PlayerSide`], [`NoteKind`], [`Key`] tuple.
    fn into_tuple(self) -> (PlayerSide, NoteKind, Key);
}
/// Trait for key channel mode implementations.
///
/// This trait defines the interface for converting between different key channel modes
/// and the standard Beat mode. Each mode implementation should provide methods to
/// convert from its own format to Beat format and vice versa.
pub trait KeyLayoutMapper: KeyMapping {
    /// Convert from this mode's format to Beat mode format.
    ///
    /// This method takes a (PlayerSide, NoteKind, Key) tuple in this mode's format and converts
    /// it to the equivalent BeatModeMap in Beat mode format.
    fn to_beat(self) -> KeyLayoutBeat;

    /// Convert from Beat mode format to this mode's format.
    ///
    /// This method takes a BeatModeMap in Beat mode format and converts
    /// it to the equivalent (PlayerSide, NoteKind, Key) tuple in this mode's format.
    fn from_beat(beat_map: KeyLayoutBeat) -> Self;
}

/// Beat 5K/7K/10K/14K, A mixture of BMS/BME type. (`16` is scratch, `17` is free zone)
/// It is the default type of key parsing.
///
/// - Lanes:
///   - Chars: '1'..'7','6' scratch, '7' free zone, '8'->Key6, '9'->Key7
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyLayoutBeat(pub PlayerSide, pub NoteKind, pub Key);

impl KeyMapping for KeyLayoutBeat {
    fn new(side: PlayerSide, kind: NoteKind, key: Key) -> Self {
        KeyLayoutBeat(side, kind, key)
    }

    fn side(&self) -> PlayerSide {
        self.0
    }

    fn kind(&self) -> NoteKind {
        self.1
    }

    fn key(&self) -> Key {
        self.2
    }

    fn into_tuple(self) -> (PlayerSide, NoteKind, Key) {
        (self.0, self.1, self.2)
    }
}

impl KeyLayoutMapper for KeyLayoutBeat {
    fn to_beat(self) -> KeyLayoutBeat {
        self
    }

    fn from_beat(beat_map: KeyLayoutBeat) -> Self {
        beat_map
    }
}

/// PMS BME-type, supports 9K/18K.
///
/// - Lanes:
///   - Chars: '1'..'9', '6'->Key8, '7'->Key9, '8'->Key6, '9'->Key7
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyLayoutPmsBmeType(pub PlayerSide, pub NoteKind, pub Key);

impl KeyMapping for KeyLayoutPmsBmeType {
    fn new(side: PlayerSide, kind: NoteKind, key: Key) -> Self {
        KeyLayoutPmsBmeType(side, kind, key)
    }

    fn side(&self) -> PlayerSide {
        self.0
    }

    fn kind(&self) -> NoteKind {
        self.1
    }

    fn key(&self) -> Key {
        self.2
    }

    fn into_tuple(self) -> (PlayerSide, NoteKind, Key) {
        (self.0, self.1, self.2)
    }
}

impl KeyLayoutMapper for KeyLayoutPmsBmeType {
    fn to_beat(self) -> KeyLayoutBeat {
        let (side, kind, key) = self.into_tuple();
        let key = match key {
            Key(8) => Scratch(1),
            Key(9) => FreeZone,
            other => other,
        };
        KeyLayoutBeat::new(side, kind, key)
    }

    fn from_beat(beat_map: KeyLayoutBeat) -> Self {
        let (side, kind, key) = beat_map.into_tuple();
        let key = match key {
            Scratch(1) => Key(8),
            FreeZone => Key(9),
            _ => key,
        };
        KeyLayoutPmsBmeType::new(side, kind, key)
    }
}

/// PMS
///
/// - Lanes:
///   - Beat -> this: (P2,Key2..Key5) remapped to (P1,Key6..Key9); (P1,Key1..Key5) unchanged
///   - This -> Beat: Key6..Key9 => (P2,Key2..Key5); Key1..Key5 => (P1,Key1..Key5)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyLayoutPms(pub PlayerSide, pub NoteKind, pub Key);

impl KeyMapping for KeyLayoutPms {
    fn new(side: PlayerSide, kind: NoteKind, key: Key) -> Self {
        KeyLayoutPms(side, kind, key)
    }

    fn side(&self) -> PlayerSide {
        self.0
    }

    fn kind(&self) -> NoteKind {
        self.1
    }

    fn key(&self) -> Key {
        self.2
    }

    fn into_tuple(self) -> (PlayerSide, NoteKind, Key) {
        (self.0, self.1, self.2)
    }
}

impl KeyLayoutMapper for KeyLayoutPms {
    fn to_beat(self) -> KeyLayoutBeat {
        use PlayerSide::*;
        let (side, kind, key) = self.into_tuple();
        let (side, key) = match (side, key) {
            (Player1, Key(1) | Key(2) | Key(3) | Key(4) | Key(5)) => (Player1, key),
            (Player1, Key(6)) => (Player2, Key(2)),
            (Player1, Key(7)) => (Player2, Key(3)),
            (Player1, Key(8)) => (Player2, Key(4)),
            (Player1, Key(9)) => (Player2, Key(5)),
            other => other,
        };
        KeyLayoutBeat::new(side, kind, key)
    }

    fn from_beat(beat_map: KeyLayoutBeat) -> Self {
        use PlayerSide::*;
        let (side, kind, key) = beat_map.into_tuple();
        let (side, key) = match (side, key) {
            (Player1, Key(1) | Key(2) | Key(3) | Key(4) | Key(5)) => (Player1, key),
            (Player2, Key(2)) => (Player1, Key(6)),
            (Player2, Key(3)) => (Player1, Key(7)),
            (Player2, Key(4)) => (Player1, Key(8)),
            (Player2, Key(5)) => (Player1, Key(9)),
            other => other,
        };
        KeyLayoutPms::new(side, kind, key)
    }
}

/// Beat nanasi/angolmois
///
/// - Lanes:
///   - Beat -> this: FreeZone=>FootPedal
///   - This -> Beat: FootPedal=>FreeZone
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyLayoutBeatNanasi(pub PlayerSide, pub NoteKind, pub Key);

impl KeyMapping for KeyLayoutBeatNanasi {
    fn new(side: PlayerSide, kind: NoteKind, key: Key) -> Self {
        KeyLayoutBeatNanasi(side, kind, key)
    }

    fn side(&self) -> PlayerSide {
        self.0
    }

    fn kind(&self) -> NoteKind {
        self.1
    }

    fn key(&self) -> Key {
        self.2
    }

    fn into_tuple(self) -> (PlayerSide, NoteKind, Key) {
        (self.0, self.1, self.2)
    }
}

impl KeyLayoutMapper for KeyLayoutBeatNanasi {
    fn to_beat(self) -> KeyLayoutBeat {
        let (side, kind, key) = self.into_tuple();
        let key = match key {
            FootPedal => FreeZone,
            other => other,
        };
        KeyLayoutBeat::new(side, kind, key)
    }

    fn from_beat(beat_map: KeyLayoutBeat) -> Self {
        let (side, kind, key) = beat_map.into_tuple();
        let key = match key {
            FreeZone => FootPedal,
            other => other,
        };
        KeyLayoutBeatNanasi::new(side, kind, key)
    }
}

/// DSC & OCT/FP
///
/// - Lanes:
///   - Beat -> this: (P2,Key1)=>FootPedal, (P2,Key2..Key7)=>Key8..Key13, (P2,Scratch)=>ScratchExtra; (P1,Key1..Key7|Scratch) unchanged; side becomes P1
///   - This -> Beat: reverse of above
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyLayoutDscOctFp(pub PlayerSide, pub NoteKind, pub Key);

impl KeyMapping for KeyLayoutDscOctFp {
    fn new(side: PlayerSide, kind: NoteKind, key: Key) -> Self {
        KeyLayoutDscOctFp(side, kind, key)
    }

    fn side(&self) -> PlayerSide {
        self.0
    }

    fn kind(&self) -> NoteKind {
        self.1
    }

    fn key(&self) -> Key {
        self.2
    }

    fn into_tuple(self) -> (PlayerSide, NoteKind, Key) {
        (self.0, self.1, self.2)
    }
}

impl KeyLayoutMapper for KeyLayoutDscOctFp {
    fn to_beat(self) -> KeyLayoutBeat {
        use PlayerSide::*;
        let (side, kind, key) = self.into_tuple();
        let (side, key) = match (side, key) {
            (
                Player1,
                Key(1) | Key(2) | Key(3) | Key(4) | Key(5) | Key(6) | Key(7) | Scratch(1),
            ) => (Player1, key),
            (Player1, Scratch(2)) => (Player2, Scratch(1)),
            (Player1, FootPedal) => (Player2, Key(1)),
            (Player1, Key(8)) => (Player2, Key(2)),
            (Player1, Key(9)) => (Player2, Key(3)),
            (Player1, Key(10)) => (Player2, Key(4)),
            (Player1, Key(11)) => (Player2, Key(5)),
            (Player1, Key(12)) => (Player2, Key(6)),
            (Player1, Key(13)) => (Player2, Key(7)),
            (s, other) => (s, other),
        };
        KeyLayoutBeat::new(side, kind, key)
    }

    fn from_beat(beat_map: KeyLayoutBeat) -> Self {
        use PlayerSide::*;
        let (side, kind, key) = beat_map.into_tuple();
        let (side, key) = match (side, key) {
            (
                Player1,
                Key(1) | Key(2) | Key(3) | Key(4) | Key(5) | Key(6) | Key(7) | Scratch(1),
            ) => (Player1, key),
            (Player2, Key(1)) => (Player1, FootPedal),
            (Player2, Key(2)) => (Player1, Key(8)),
            (Player2, Key(3)) => (Player1, Key(9)),
            (Player2, Key(4)) => (Player1, Key(10)),
            (Player2, Key(5)) => (Player1, Key(11)),
            (Player2, Key(6)) => (Player1, Key(12)),
            (Player2, Key(7)) => (Player1, Key(13)),
            (Player2, Scratch(1)) => (Player1, Scratch(2)),
            (s, k) => (s, k),
        };
        KeyLayoutDscOctFp::new(side, kind, key)
    }
}
