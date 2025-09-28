//! Prompting interface and utilities.
//!
//! An object implementing [`Prompter`] is required by [`super::Bms::from_token_stream`]. It is used to handle conflicts and prompt workarounds on parsing the BMS file.

use std::path::Path;

use crate::bms::{
    Decimal,
    command::{
        ObjId,
        channel::Channel,
        time::{ObjTime, Track},
    },
};

use super::{ParseWarning, Result};
use crate::bms::{
    model::def::{AtBgaDef, BgaDef, Bmp, ExRankDef},
    model::obj::{
        BgaObj, BgmVolumeObj, BpmChangeObj, JudgeObj, KeyVolumeObj, ScrollingFactorObj,
        SectionLenChangeObj, SpeedObj, TextObj,
    },
};

#[cfg(feature = "minor-command")]
use crate::bms::{
    command::{
        graphics::Argb,
        minor_command::{StpEvent, SwBgaEvent, WavCmdEvent},
    },
    model::{
        def::ExWavDef,
        obj::{BgaArgbObj, BgaKeyboundObj, BgaOpacityObj, OptionObj, SeekObj},
    },
};

/// An interface to prompt about handling conflicts on the BMS file.
pub trait Prompter {
    /// Determines a [`DuplicationWorkaround`] for [`DefDuplication`].
    fn handle_def_duplication(&self, duplication: DefDuplication) -> DuplicationWorkaround;
    /// Determines a [`DuplicationWorkaround`] for [`TrackDuplication`].
    fn handle_track_duplication(&self, duplication: TrackDuplication) -> DuplicationWorkaround;
    /// Determines a [`DuplicationWorkaround`] for [`ChannelDuplication`].
    fn handle_channel_duplication(&self, duplication: ChannelDuplication) -> DuplicationWorkaround;
    /// Shows the user a [`ParseWarning`].
    fn warn(&self, warning: ParseWarning) {
        eprintln!("{warning:?}");
    }
}

/// It represents that there is a duplicated definition on the BMS file.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DefDuplication<'a> {
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
    /// BGA ARGB color definition is duplicated.
    #[cfg(feature = "minor-command")]
    BgaArgb {
        /// Duplicated BGA ARGB object id.
        id: ObjId,
        /// Existing definition.
        older: &'a Argb,
        /// Incoming definition.
        newer: &'a Argb,
    },
    /// `WAVCMD` event is duplicated.
    #[cfg(feature = "minor-command")]
    WavCmdEvent {
        /// Duplicated `WAVCMD` event `wav_index`.
        wav_index: ObjId,
        /// Existing definition.
        older: &'a WavCmdEvent,
        /// Incoming definition.
        newer: &'a WavCmdEvent,
    },
    /// SWBGA event is duplicated.
    #[cfg(feature = "minor-command")]
    SwBgaEvent {
        /// Duplicated SWBGA event id.
        id: ObjId,
        /// Existing definition.
        older: &'a SwBgaEvent,
        /// Incoming definition.
        newer: &'a SwBgaEvent,
    },
    /// Seek event is duplicated.
    #[cfg(feature = "minor-command")]
    SeekEvent {
        /// Duplicated Seek event id.
        id: ObjId,
        /// Existing definition.
        older: &'a Decimal,
        /// Incoming definition.
        newer: &'a Decimal,
    },
}

/// It represents that there is a duplicated track object on the BMS file.
pub enum TrackDuplication<'a> {
    /// Section length change event is duplicated.
    SectionLenChangeEvent {
        /// Duplicated section length change track.
        track: Track,
        /// Existing definition.
        older: &'a SectionLenChangeObj,
        /// Incoming definition.
        newer: &'a SectionLenChangeObj,
    },
}

/// It represents that there is a duplicated channel object on the BMS file.
pub enum ChannelDuplication<'a> {
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
        older: &'a SpeedObj,
        /// Incoming definition.
        newer: &'a SpeedObj,
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
    /// BGA opacity change event is duplicated.
    #[cfg(feature = "minor-command")]
    BgaOpacityChangeEvent {
        /// Duplicated BGA opacity change time.
        time: ObjTime,
        /// Existing definition.
        older: &'a BgaOpacityObj,
        /// Incoming definition.
        newer: &'a BgaOpacityObj,
    },
    /// BGA ARGB color change event is duplicated.
    #[cfg(feature = "minor-command")]
    BgaArgbChangeEvent {
        /// Duplicated BGA ARGB change time.
        time: ObjTime,
        /// Existing definition.
        older: &'a BgaArgbObj,
        /// Incoming definition.
        newer: &'a BgaArgbObj,
    },
    /// STP event is duplicated.
    #[cfg(feature = "minor-command")]
    StpEvent {
        /// Duplicated STP event time.
        time: ObjTime,
        /// Existing definition.
        older: &'a StpEvent,
        /// Incoming definition.
        newer: &'a StpEvent,
    },
    /// BGM volume change event is duplicated.
    BgmVolumeChangeEvent {
        /// Duplicated BGM volume change time.
        time: ObjTime,
        /// Existing definition.
        older: &'a BgmVolumeObj,
        /// Incoming definition.
        newer: &'a BgmVolumeObj,
    },
    /// KEY volume change event is duplicated.
    KeyVolumeChangeEvent {
        /// Duplicated KEY volume change time.
        time: ObjTime,
        /// Existing definition.
        older: &'a KeyVolumeObj,
        /// Incoming definition.
        newer: &'a KeyVolumeObj,
    },
    /// Seek message event is duplicated.
    #[cfg(feature = "minor-command")]
    SeekMessageEvent {
        /// Duplicated seek time.
        time: ObjTime,
        /// Existing definition.
        older: &'a SeekObj,
        /// Incoming definition.
        newer: &'a SeekObj,
    },
    /// Text event is duplicated.
    TextEvent {
        /// Duplicated text time.
        time: ObjTime,
        /// Existing definition.
        older: &'a TextObj,
        /// Incoming definition.
        newer: &'a TextObj,
    },
    /// Judge event is duplicated.
    JudgeEvent {
        /// Duplicated judge time.
        time: ObjTime,
        /// Existing definition.
        older: &'a JudgeObj,
        /// Incoming definition.
        newer: &'a JudgeObj,
    },
    /// BGA keybound event is duplicated.
    #[cfg(feature = "minor-command")]
    BgaKeyboundEvent {
        /// Duplicated BGA keybound time.
        time: ObjTime,
        /// Existing definition.
        older: &'a BgaKeyboundObj,
        /// Incoming definition.
        newer: &'a BgaKeyboundObj,
    },
    /// Option event is duplicated.
    #[cfg(feature = "minor-command")]
    OptionEvent {
        /// Duplicated option time.
        time: ObjTime,
        /// Existing definition.
        older: &'a OptionObj,
        /// Incoming definition.
        newer: &'a OptionObj,
    },
}

/// A choice to handle the duplicated definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DuplicationWorkaround {
    /// Choose to use the existing one.
    UseOlder,
    /// Choose to use the incoming one.
    UseNewer,
    /// Choose to warn and use older values.
    WarnAndUseOlder,
    /// Choose to warn and use newer values.
    WarnAndUseNewer,
}

impl DuplicationWorkaround {
    pub(crate) fn apply_def<T>(self, target: &mut T, newer: T, id: ObjId) -> Result<()> {
        match self {
            Self::UseOlder => Ok(()),
            Self::UseNewer => {
                *target = newer;
                Ok(())
            }
            Self::WarnAndUseOlder => Err(ParseWarning::DuplicatingDef(id)),
            Self::WarnAndUseNewer => {
                *target = newer;
                Err(ParseWarning::DuplicatingDef(id))
            }
        }
    }

    pub(crate) fn apply_track<T>(
        self,
        target: &mut T,
        newer: T,
        track: Track,
        channel: Channel,
    ) -> Result<()> {
        match self {
            Self::UseOlder => Ok(()),
            Self::UseNewer => {
                *target = newer;
                Ok(())
            }
            Self::WarnAndUseOlder => Err(ParseWarning::DuplicatingTrackObj(track, channel)),
            Self::WarnAndUseNewer => {
                *target = newer;
                Err(ParseWarning::DuplicatingTrackObj(track, channel))
            }
        }
    }

    pub(crate) fn apply_channel<T>(
        self,
        target: &mut T,
        newer: T,
        time: ObjTime,
        channel: Channel,
    ) -> Result<()> {
        match self {
            Self::UseOlder => Ok(()),
            Self::UseNewer => {
                *target = newer;
                Ok(())
            }
            Self::WarnAndUseOlder => Err(ParseWarning::DuplicatingChannelObj(time, channel)),
            Self::WarnAndUseNewer => {
                *target = newer;
                Err(ParseWarning::DuplicatingChannelObj(time, channel))
            }
        }
    }
}

/// The strategy that always using older ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlwaysUseOlder;

impl Prompter for AlwaysUseOlder {
    fn handle_def_duplication(&self, _: DefDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::UseOlder
    }

    fn handle_track_duplication(&self, _: TrackDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::UseOlder
    }

    fn handle_channel_duplication(&self, _: ChannelDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::UseOlder
    }
}

/// The strategy that always using newer ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlwaysUseNewer;

impl Prompter for AlwaysUseNewer {
    fn handle_def_duplication(&self, _: DefDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::UseNewer
    }

    fn handle_track_duplication(&self, _: TrackDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::UseNewer
    }

    fn handle_channel_duplication(&self, _: ChannelDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::UseNewer
    }
}

/// The strategy that always warns and uses older values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlwaysWarnAndUseOlder;

impl Prompter for AlwaysWarnAndUseOlder {
    fn handle_def_duplication(&self, _: DefDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::WarnAndUseOlder
    }

    fn handle_track_duplication(&self, _: TrackDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::WarnAndUseOlder
    }

    fn handle_channel_duplication(&self, _: ChannelDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::WarnAndUseOlder
    }
}

/// The strategy that always warns and uses newer values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlwaysWarnAndUseNewer;

impl Prompter for AlwaysWarnAndUseNewer {
    fn handle_def_duplication(&self, _: DefDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::WarnAndUseNewer
    }

    fn handle_track_duplication(&self, _: TrackDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::WarnAndUseNewer
    }

    fn handle_channel_duplication(&self, _: ChannelDuplication) -> DuplicationWorkaround {
        DuplicationWorkaround::WarnAndUseNewer
    }
}
