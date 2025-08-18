//! For converting key/channel between different modes, please see [`KeyLayoutMapper`] enum and [`convert_key_mapping_between`] function.

use super::{Key, KeyMapping, PlayerSide};

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
    fn map_to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping;

    /// Convert from Beat mode format to this mode's format.
    ///
    /// This method takes a BeatModeMap in Beat mode format and converts
    /// it to the equivalent (PlayerSide, Key) pair in this mode's format.
    fn map_from_beat(&mut self, beat_map: KeyMapping) -> KeyMapping;
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
    let beat_map = src.map_to_beat(beat_map);
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
    fn map_to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
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
    fn map_to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
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
    fn map_to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
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
    fn map_to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
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
    fn map_to_beat(&mut self, beat_map: KeyMapping) -> KeyMapping {
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
