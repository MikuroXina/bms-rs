//! For converting key/channel between different modes, please see [`KeyLayoutMapper`] enum and [`convert_key_mapping_between`] function.

use super::{Key, NoteChannelId, NoteKind, PlayerSide};
use Key::*;

/// Convert from [`KeyLayoutBeat`] to [`ChannelId`].
fn key_layout_beat_to_channel_id(beat: KeyLayoutBeat) -> NoteChannelId {
    let (side, kind, key) = beat.as_tuple();

    // First character based on NoteKind and PlayerSide
    let first_char = match (kind, side) {
        (NoteKind::Visible, PlayerSide::Player1) => '1',
        (NoteKind::Visible, PlayerSide::Player2) => '2',
        (NoteKind::Invisible, PlayerSide::Player1) => '3',
        (NoteKind::Invisible, PlayerSide::Player2) => '4',
        (NoteKind::Long, PlayerSide::Player1) => '5',
        (NoteKind::Long, PlayerSide::Player2) => '6',
        (NoteKind::Landmine, PlayerSide::Player1) => 'D',
        (NoteKind::Landmine, PlayerSide::Player2) => 'E',
    };

    // Second character based on Key
    let second_char = match key {
        Key::Key(1) => '1',
        Key::Key(2) => '2',
        Key::Key(3) => '3',
        Key::Key(4) => '4',
        Key::Key(5) => '5',
        Key::Scratch(1) => '6',
        Key::FreeZone => '7',
        Key::Key(6) => '8',
        Key::Key(7) => '9',
        _ => '1', // Default fallback
    };

    NoteChannelId::try_from([first_char as u8, second_char as u8]).unwrap()
}

/// Convert from [`ChannelId`] to [`KeyLayoutBeat`].
fn channel_id_to_key_layout_beat(channel_id: NoteChannelId) -> Option<KeyLayoutBeat> {
    let chars = channel_id.0.map(|c| c as char);
    let first_char = chars[0];
    let second_char = chars[1];

    // Parse NoteKind and PlayerSide from first character
    let (kind, side) = match first_char {
        '1' => (NoteKind::Visible, PlayerSide::Player1),
        '2' => (NoteKind::Visible, PlayerSide::Player2),
        '3' => (NoteKind::Invisible, PlayerSide::Player1),
        '4' => (NoteKind::Invisible, PlayerSide::Player2),
        '5' => (NoteKind::Long, PlayerSide::Player1),
        '6' => (NoteKind::Long, PlayerSide::Player2),
        'D' => (NoteKind::Landmine, PlayerSide::Player1),
        'E' => (NoteKind::Landmine, PlayerSide::Player2),
        _ => return None,
    };

    // Parse Key from second character
    let key = match second_char {
        '1' => Key::Key(1),
        '2' => Key::Key(2),
        '3' => Key::Key(3),
        '4' => Key::Key(4),
        '5' => Key::Key(5),
        '6' => Key::Scratch(1),
        '7' => Key::FreeZone,
        '8' => Key::Key(6),
        '9' => Key::Key(7),
        _ => return None,
    };

    Some(KeyLayoutBeat::new(side, kind, key))
}

/// A trait for key mapping storage structure.
pub trait KeyMapping {
    /// Create a new [`KeyMapping`] from a [`PlayerSide`], [`NoteKind`] and [`Key`].
    fn new(side: PlayerSide, kind: NoteKind, key: Key) -> Self;
    /// Get the [`PlayerSide`] from this [`KeyMapping`].
    fn side(&self) -> PlayerSide;
    /// Get the [`NoteKind`] from this [`KeyMapping`].
    fn kind(&self) -> NoteKind;
    /// Get the [`Key`] from this [`KeyMapping`].
    fn key(&self) -> Key;
    /// Create a new [`KeyMapping`] from a tuple of [`PlayerSide`], [`NoteKind`] and [`Key`].
    fn from_tuple(tuple: (PlayerSide, NoteKind, Key)) -> Self
    where
        Self: Sized,
    {
        Self::new(tuple.0, tuple.1, tuple.2)
    }
    /// Deconstruct into a [`PlayerSide`], [`NoteKind`], [`Key`] tuple.
    fn as_tuple(&self) -> (PlayerSide, NoteKind, Key) {
        (self.side(), self.kind(), self.key())
    }
}

/// Trait for key channel mode implementations.
///
/// This trait defines the interface for converting between different key channel modes
/// and the standard [`ChannelId`] format. Each mode implementation should provide methods to
/// convert from its own format to [`ChannelId`] format and vice versa.
pub trait KeyLayoutMapper: KeyMapping {
    /// Convert from this mode's format to [`ChannelId`] format.
    ///
    /// This method takes a ([`PlayerSide`], [`NoteKind`], [`Key`]) tuple in this mode's format and converts
    /// it to the equivalent [`ChannelId`] in [`ChannelId`] format.
    fn to_channel_id(self) -> NoteChannelId;

    /// Convert from [`ChannelId`] format to this mode's format.
    ///
    /// This method takes a [`ChannelId`] in [`ChannelId`] format and converts
    /// it to the equivalent ([`PlayerSide`], [`NoteKind`], [`Key`]) tuple in this mode's format.
    fn from_channel_id(channel_id: NoteChannelId) -> Option<Self>
    where
        Self: Sized;
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
        Self(side, kind, key)
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
}

impl KeyLayoutMapper for KeyLayoutBeat {
    fn to_channel_id(self) -> NoteChannelId {
        key_layout_beat_to_channel_id(self)
    }

    fn from_channel_id(channel_id: NoteChannelId) -> Option<Self> {
        channel_id_to_key_layout_beat(channel_id)
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
        Self(side, kind, key)
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
}

impl KeyLayoutMapper for KeyLayoutPmsBmeType {
    fn to_channel_id(self) -> NoteChannelId {
        let (side, kind, key) = self.as_tuple();
        let key = match key {
            Key(8) => Scratch(1),
            Key(9) => FreeZone,
            other => other,
        };
        let beat = KeyLayoutBeat::new(side, kind, key);
        key_layout_beat_to_channel_id(beat)
    }

    fn from_channel_id(channel_id: NoteChannelId) -> Option<Self> {
        let beat = channel_id_to_key_layout_beat(channel_id)?;
        let (side, kind, key) = beat.as_tuple();
        let key = match key {
            Scratch(1) => Key(8),
            FreeZone => Key(9),
            _ => key,
        };
        Some(Self::new(side, kind, key))
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
        Self(side, kind, key)
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
}

impl KeyLayoutMapper for KeyLayoutPms {
    fn to_channel_id(self) -> NoteChannelId {
        use PlayerSide::*;
        let (side, kind, key) = self.as_tuple();
        let (side, key) = match (side, key) {
            (Player1, Key(1..=5)) => (Player1, key),
            (Player1, Key(6)) => (Player2, Key(2)),
            (Player1, Key(7)) => (Player2, Key(3)),
            (Player1, Key(8)) => (Player2, Key(4)),
            (Player1, Key(9)) => (Player2, Key(5)),
            other => other,
        };
        let beat = KeyLayoutBeat::new(side, kind, key);
        key_layout_beat_to_channel_id(beat)
    }

    fn from_channel_id(channel_id: NoteChannelId) -> Option<Self> {
        use PlayerSide::*;
        let beat = channel_id_to_key_layout_beat(channel_id)?;
        let (side, kind, key) = beat.as_tuple();
        let (side, key) = match (side, key) {
            (Player1, Key(1..=5)) => (Player1, key),
            (Player2, Key(2)) => (Player1, Key(6)),
            (Player2, Key(3)) => (Player1, Key(7)),
            (Player2, Key(4)) => (Player1, Key(8)),
            (Player2, Key(5)) => (Player1, Key(9)),
            other => other,
        };
        Some(Self::new(side, kind, key))
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
        Self(side, kind, key)
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
}

impl KeyLayoutMapper for KeyLayoutBeatNanasi {
    fn to_channel_id(self) -> NoteChannelId {
        let (side, kind, key) = self.as_tuple();
        let key = match key {
            FootPedal => FreeZone,
            other => other,
        };
        let beat = KeyLayoutBeat::new(side, kind, key);
        key_layout_beat_to_channel_id(beat)
    }

    fn from_channel_id(channel_id: NoteChannelId) -> Option<Self> {
        let beat = channel_id_to_key_layout_beat(channel_id)?;
        let (side, kind, key) = beat.as_tuple();
        let key = match key {
            FreeZone => FootPedal,
            other => other,
        };
        Some(Self::new(side, kind, key))
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
        Self(side, kind, key)
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
}

impl KeyLayoutMapper for KeyLayoutDscOctFp {
    fn to_channel_id(self) -> NoteChannelId {
        use PlayerSide::*;
        let (side, kind, key) = self.as_tuple();
        let (side, key) = match (side, key) {
            (Player1, Key(1..=7) | Scratch(1)) => (Player1, key),
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
        let beat = KeyLayoutBeat::new(side, kind, key);
        key_layout_beat_to_channel_id(beat)
    }

    fn from_channel_id(channel_id: NoteChannelId) -> Option<Self> {
        use PlayerSide::*;
        let beat = channel_id_to_key_layout_beat(channel_id)?;
        let (side, kind, key) = beat.as_tuple();
        let (side, key) = match (side, key) {
            (Player1, Key(1..=7) | Scratch(1)) => (Player1, key),
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
        Some(Self::new(side, kind, key))
    }
}
