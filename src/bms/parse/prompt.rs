//! Prompting interface and utilities.
//!
//! An object implementing [`PromptHandler`] is required by [`super::Bms::from_token_stream`]. It is used to handle conflicts and prompt workarounds on parsing the BMS file.

use std::path::Path;

use crate::bms::{Decimal, command::*};

#[cfg(feature = "minor-command")]
use super::model::def::ExWavDef;
use super::{
    ParseWarning, Result,
    model::def::{AtBgaDef, BgaDef, Bmp, ExRankDef},
    model::obj::{BgaObj, BpmChangeObj, ScrollingFactorObj, SectionLenChangeObj, SpeedFactorObj},
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
        older: Decimal,
        /// Incoming definition.
        newer: Decimal,
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
    SpeedFactorChange {
        /// Duplicated SPEED object id.
        id: ObjId,
        /// Existing definition.
        older: Decimal,
        /// Incoming definition.
        newer: Decimal,
    },
    /// SCROLL definition is duplicated.
    ScrollingFactorChange {
        /// Duplicated SCROLL object id.
        id: ObjId,
        /// Existing definition.
        older: Decimal,
        /// Incoming definition.
        newer: Decimal,
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
    #[cfg(feature = "minor-command")]
    ExWav {
        /// Duplicated EXWAV object id.
        id: ObjId,
        /// Existing definition.
        older: &'a ExWavDef,
        /// Incoming definition.
        newer: &'a ExWavDef,
    },
    /// STOP definition is duplicated.
    Stop {
        /// Duplicated STOP object id.
        id: ObjId,
        /// Existing definition.
        older: Decimal,
        /// Incoming definition.
        newer: Decimal,
    },
    /// BPM change event is duplicated.
    BpmChangeEvent {
        /// Duplicated BPM change time.
        time: ObjTime,
        /// Existing definition.
        older: &'a BpmChangeObj,
        /// Incoming definition.
        newer: &'a BpmChangeObj,
    },
    /// Scrolling factor change event is duplicated.
    ScrollingFactorChangeEvent {
        /// Duplicated scrolling factor change time.
        time: ObjTime,
        /// Existing definition.
        older: &'a ScrollingFactorObj,
        /// Incoming definition.
        newer: &'a ScrollingFactorObj,
    },
    /// Speed factor change event is duplicated.
    SpeedFactorChangeEvent {
        /// Duplicated speed factor change time.
        time: ObjTime,
        /// Existing definition.
        older: &'a SpeedFactorObj,
        /// Incoming definition.
        newer: &'a SpeedFactorObj,
    },
    /// Section length change event is duplicated.
    SectionLenChangeEvent {
        /// Duplicated section length change track.
        track: Track,
        /// Existing definition.
        older: &'a SectionLenChangeObj,
        /// Incoming definition.
        newer: &'a SectionLenChangeObj,
    },
    /// BGA change event is duplicated.
    BgaChangeEvent {
        /// Duplicated BGA change time.
        time: ObjTime,
        /// Existing definition.
        older: &'a BgaObj,
        /// Incoming definition.
        newer: &'a BgaObj,
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
    /// Choose to warn.
    Warn,
}

impl DuplicationWorkaround {
    pub(crate) fn apply<T: Clone>(self, target: &mut T, newer: T) -> Result<()> {
        match self {
            DuplicationWorkaround::UseOlder => Ok(()),
            DuplicationWorkaround::UseNewer => {
                *target = newer;
                Ok(())
            }
            DuplicationWorkaround::Warn => Err(ParseWarning::PromptHandlerWarning),
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

/// The strategy that always warns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlwaysWarn;

impl PromptHandler for AlwaysWarn {
    fn handle_duplication(&mut self, _: PromptingDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::Warn
    }
}
