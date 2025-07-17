//! Prompting interface and utilities.
//!
//! An object implementing [`PromptHandler`] is required by [`super::Bms::from_token_stream`]. It is used to handle conflicts and prompt workarounds on parsing the BMS file.

use std::path::Path;

use crate::bms::lex::command::ObjId;

use super::{
    ParseError, Result,
    header::{AtBgaDef, BgaDef, Bmp, ExRankDef, ExWavDef},
};

/// An interface to prompt about handling conflicts on the BMS file.
pub trait PromptHandler {
    /// Determines a [`DuplicationWorkaround`] for duplicating conflicts.
    fn handle_duplication(&mut self, duplication: PromptingDuplication) -> DuplicationWorkaround;
}

/// It represents that there is a duplicated definition on the BMS file.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PromptingDuplication<'a> {
    /// BMP definition is duplicated.
    Bmp {
        /// Duplicated BMP object id.
        id: ObjId,
        /// Existing definition.
        older: &'a Bmp,
        /// Incoming definition.
        newer: &'a Bmp,
    },
    /// BPM definition is duplicated.
    BpmChange {
        /// Duplicated BPM object id.
        id: ObjId,
        /// Existing definition.
        older: f64,
        /// Incoming definition.
        newer: f64,
    },
    /// OPTION definition is duplicated.
    ChangeOption {
        /// Duplicated OPTION object id.
        id: ObjId,
        /// Existing definition.
        older: &'a str,
        /// Incoming definition.
        newer: &'a str,
    },
    /// SPEED definition is duplicated.
    SpacingFactorChange {
        /// Duplicated SPEED object id.
        id: ObjId,
        /// Existing definition.
        older: f64,
        /// Incoming definition.
        newer: f64,
    },
    /// SCROLL definition is duplicated.
    ScrollingFactorChange {
        /// Duplicated SCROLL object id.
        id: ObjId,
        /// Existing definition.
        older: f64,
        /// Incoming definition.
        newer: f64,
    },
    /// TEXT is duplicated.
    Text {
        /// Duplicated TEXT object id.
        id: ObjId,
        /// Existing definition.
        older: &'a str,
        /// Incoming definition.
        newer: &'a str,
    },
    /// WAV definition is duplicated.
    Wav {
        /// Duplicated WAV object id.
        id: ObjId,
        /// Existing definition.
        older: &'a Path,
        /// Incoming definition.
        newer: &'a Path,
    },
    /// @BGA definition is duplicated.
    AtBga {
        /// Duplicated @BGA object id.
        id: ObjId,
        /// Existing definition.
        older: &'a AtBgaDef,
        /// Incoming definition.
        newer: &'a AtBgaDef,
    },
    /// BGA definition is duplicated.
    Bga {
        /// Duplicated BGA object id.
        id: ObjId,
        /// Existing definition.
        older: &'a BgaDef,
        /// Incoming definition.
        newer: &'a BgaDef,
    },
    /// EXRANK definition is duplicated.
    ExRank {
        /// Duplicated EXRANK object id.
        id: ObjId,
        /// Existing definition.
        older: &'a ExRankDef,
        /// Incoming definition.
        newer: &'a ExRankDef,
    },
    /// EXWAV definition is duplicated.
    ExWav {
        /// Duplicated EXWAV object id.
        id: ObjId,
        /// Existing definition.
        older: &'a ExWavDef,
        /// Incoming definition.
        newer: &'a ExWavDef,
    },
}

/// A choice to handle the duplicated definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DuplicationWorkaround {
    /// Choose to use the existing one.
    UseOlder,
    /// Choose to use the incoming one.
    UseNewer,
    /// Choose to interrupt this parsing.
    Halt,
}

impl DuplicationWorkaround {
    pub(crate) fn apply<T>(self, target: &mut T, newer: T) -> Result<()> {
        match self {
            DuplicationWorkaround::UseOlder => Ok(()),
            DuplicationWorkaround::UseNewer => {
                *target = newer;
                Ok(())
            }
            DuplicationWorkaround::Halt => Err(ParseError::Halted),
        }
    }
}

/// The strategy that always using older ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlwaysUseOlder;

impl PromptHandler for AlwaysUseOlder {
    fn handle_duplication(&mut self, _: PromptingDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::UseOlder
    }
}

/// The strategy that always using newer ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlwaysUseNewer;

impl PromptHandler for AlwaysUseNewer {
    fn handle_duplication(&mut self, _: PromptingDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::UseNewer
    }
}

/// The strategy that always halts parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlwaysHalt;

impl PromptHandler for AlwaysHalt {
    fn handle_duplication(&mut self, _: PromptingDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::Halt
    }
}
