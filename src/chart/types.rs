//! Chart-level types for describing notes and player interactions.
//!
//! These types describe fundamental concepts used in charts,
//! independent of any specific file format (BMS, BMSON, etc.).

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
    /// Returns whether the note is a displayable.
    #[must_use]
    pub const fn is_displayable(self) -> bool {
        !matches!(self, Self::Invisible)
    }

    /// Returns whether the note is a playable.
    #[must_use]
    pub const fn is_playable(self) -> bool {
        matches!(self, Self::Visible | Self::Long)
    }

    /// Returns whether the note is a long-press note.
    #[must_use]
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
pub enum Key {
    /// The keys for the controller.
    Key(u8),
    /// The scratch disk.
    Scratch(u8),
    /// The foot pedal.
    FootPedal,
    /// The zone that the user can scratch disk freely.
    /// `17` in BMS-type Player1.
    FreeZone,
}

impl Key {
    /// Returns whether the key expected a piano keyboard.
    #[must_use]
    pub const fn is_keyxx(&self) -> bool {
        matches!(self, Self::Key(_))
    }

    /// Returns the key number if it's a Key variant.
    #[must_use]
    pub const fn key_number(&self) -> Option<u8> {
        if let Self::Key(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// Returns the scratch number if it's a Scratch variant.
    #[must_use]
    pub const fn scratch_number(&self) -> Option<u8> {
        if let Self::Scratch(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// Creates a Key variant with the given number.
    #[must_use]
    pub const fn new_key(n: u8) -> Self {
        Self::Key(n)
    }

    /// Creates a Scratch variant with the given number.
    #[must_use]
    pub const fn new_scratch(n: u8) -> Self {
        Self::Scratch(n)
    }
}

/// BGA layer type (shared between BMS and BMSON).
/// Order matches existing `BmsLayer` definition: Base, Poor, Overlay, Overlay2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BgaLayer {
    /// The lowest layer.
    Base,
    /// Layer which is displayed only if a player missed to play notes.
    Poor,
    /// An overlaying layer.
    Overlay,
    /// An overlaying layer layered over `Overlay`.
    Overlay2,
}

/// ARGB color type (shared between BMS and BMSON).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Argb {
    /// Alpha component.
    pub alpha: u8,
    /// Red component.
    pub red: u8,
    /// Green component.
    pub green: u8,
    /// Blue component.
    pub blue: u8,
}

impl Default for Argb {
    fn default() -> Self {
        Self {
            alpha: 255,
            red: 0,
            green: 0,
            blue: 0,
        }
    }
}
