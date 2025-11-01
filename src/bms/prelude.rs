//! Prelude module for the BMS crate.
//!
//! This module re-exports all public types from the BMS module for convenient access.
//! You can use `use bms_rs::bms::prelude::*;` to import all BMS types at once.

// Re-export diagnostics from bms level
#[cfg(feature = "diagnostics")]
pub use crate::diagnostics::{SimpleSource, ToAriadne, emit_bms_warnings};

// Re-export types from bms module
pub use super::{
    BmsOutput, BmsWarning, Decimal, ParseConfig,
    command::{
        JudgeLevel, LnMode, LnType, ObjId, ObjIdManager, PlayerMode, PoorMode, Volume,
        channel::{
            Channel, Key, NoteChannelId, NoteKind, PlayerSide,
            converter::{
                KeyConverter, KeyMappingConvertFlip, KeyMappingConvertLaneRandomShuffle,
                KeyMappingConvertLaneRotateShuffle, KeyMappingConvertMirror,
                PlayerSideKeyConverter,
            },
            mapper::{
                KeyLayoutBeat, KeyLayoutBeatNanasi, KeyLayoutDscOctFp, KeyLayoutMapper,
                KeyLayoutPms, KeyLayoutPmsBmeType, KeyMapping,
            },
            read_channel,
        },
        graphics::{Argb, PixelPoint, PixelSize, Rgb},
        minor_command::{
            ExWavFrequency, ExWavPan, ExWavVolume, ExtChrEvent, StpEvent, SwBgaEvent, WavCmdEvent,
            WavCmdParam,
        },
        mixin::{SourceRangeMixin, SourceRangeMixinExt},
        time::{ObjTime, Track},
    },
    default_config, default_config_with_rng,
    error::{ParseWarning, ParseWarningWithRange},
    lex::{
        LexOutput, LexWarning, TokenRefStream, TokenStream,
        cursor::Cursor,
        token::{Token, TokenWithRange},
    },
    model::{
        Bms, Notes,
        bmp::{AtBgaDef, BgaDef, Bmp},
        judge::ExRankDef,
        obj::{
            BgaArgbObj, BgaKeyboundObj, BgaLayer, BgaObj, BgaOpacityObj, BgmVolumeObj,
            BpmChangeObj, JudgeObj, KeyVolumeObj, OptionObj, ScrollingFactorObj,
            SectionLenChangeObj, SeekObj, SpeedObj, StopObj, TextObj, WavObj,
        },
        wav::ExWavDef,
    },
    parse::{
        ParseOutput,
        check_playing::{PlayingCheckOutput, PlayingError, PlayingWarning},
        prompt::{
            AlwaysUseNewer, AlwaysUseOlder, AlwaysWarnAndUseNewer, AlwaysWarnAndUseOlder,
            DefDuplication, DuplicationWorkaround, Prompter,
        },
        validity::{ValidityCheckOutput, ValidityInvalid, ValidityMissing},
    },
    parse_bms,
    rng::{Rng, RngMock},
};

// Re-export related members when `rand` feature is enabled
#[cfg(feature = "rand")]
pub use super::rng::RandRng;
