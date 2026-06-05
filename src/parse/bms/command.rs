//! Definitions of command argument data.
//!
//! Structures in this module can be used in `Lex` part, `Parse` part, and the output models.

use super::parse::ParseWarning;

pub mod channel;
pub mod graphics;
/// Minor command types and utilities.
///
/// This module contains types and utilities for minor BMS commands that are only available
/// when the `minor-command` feature is enabled.
pub mod minor_command;
pub mod mixin;

/// A play style of the score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlayerMode {
    /// For single play, a player uses 5 or 7 keys.
    Single,
    /// For couple play, two players use each 5 or 7 keys.
    Two,
    /// For double play, a player uses 10 or 14 keys.
    Double,
}

impl std::fmt::Display for PlayerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single => write!(f, "1"),
            Self::Two => write!(f, "2"),
            Self::Double => write!(f, "3"),
        }
    }
}

impl std::str::FromStr for PlayerMode {
    type Err = ParseWarning;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "1" => Self::Single,
            "2" => Self::Two,
            "3" => Self::Double,
            _ => {
                return Err(ParseWarning::SyntaxError(
                    "expected one of 0, 1 or 2".into(),
                ));
            }
        })
    }
}

/// A play volume of the sound in the score. Defaults to 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Volume {
    /// A play volume percentage of the sound.
    pub relative_percent: u8,
}

impl Default for Volume {
    fn default() -> Self {
        Self {
            relative_percent: 100,
        }
    }
}

/// A POOR BGA display mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PoorMode {
    /// To hide the normal BGA and display the POOR BGA.
    #[default]
    Interrupt,
    /// To overlap the POOR BGA onto the normal BGA.
    Overlay,
    /// Not to display the POOR BGA.
    Hidden,
}

impl std::str::FromStr for PoorMode {
    type Err = ParseWarning;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "0" => Self::Interrupt,
            "1" => Self::Overlay,
            "2" => Self::Hidden,
            _ => {
                return Err(ParseWarning::SyntaxError(
                    "expected one of 0, 1 or 2".into(),
                ));
            }
        })
    }
}

impl PoorMode {
    /// Converts an display type of Poor BGA into the corresponding string literal.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Interrupt => "0",
            Self::Overlay => "1",
            Self::Hidden => "2",
        }
    }
}

/// A notation type about LN in the score. But you don't have to take care of how the notes are actually placed in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LnType {
    /// The RDM type.
    #[default]
    Rdm,
    /// The MGQ type.
    Mgq,
}

/// Long Note Mode Type
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum LnMode {
    /// Normal Long Note, no tail judge (LN)
    #[default]
    Ln = 1,
    /// IIDX Classic Long Note, with tail judge (CN)
    Cn = 2,
    /// IIDX Hell Long Note, with tail judge. holding add gurge, un-holding lose gurge (HCN)
    Hcn = 3,
}

impl From<LnMode> for u8 {
    fn from(mode: LnMode) -> Self {
        match mode {
            LnMode::Ln => 1,
            LnMode::Cn => 2,
            LnMode::Hcn => 3,
        }
    }
}

impl TryFrom<u8> for LnMode {
    type Error = u8;
    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::Ln,
            2 => Self::Cn,
            3 => Self::Hcn,
            _ => return Err(value),
        })
    }
}
