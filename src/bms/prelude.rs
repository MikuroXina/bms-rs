//! Prelude module for the BMS crate.
//!
//! This module re-exports all public types from the BMS module for convenient access.
//! You can use `use bms_rs::bms::prelude::*;` to import all BMS types at once.

// Re-export diagnostics from bms level
pub use crate::diagnostics::{SimpleSource, ToAriadne, emit_bms_warnings};

// Re-export types from bms module
pub use super::{
    BmsOutput, BmsWarning, Decimal,
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
        mixin::{SourceRangeMixin, SourceRangeMixinExt},
        time::{ObjTime, Track},
    },
    default_preset, default_preset_with_prompter, default_preset_with_rng,
    lex::{
        LexOutput, LexWarning, TokenRefStream, TokenStream,
        cursor::Cursor,
        token::{Token, TokenWithRange},
    },
    model::{
        Arrangers, Bms, Graphics, Header, Notes, Others, ScopeDefines,
        def::{AtBgaDef, BgaDef, Bmp, ExRankDef},
        obj::{
            BgaLayer, BgaObj, BgmVolumeObj, BpmChangeObj, JudgeObj, KeyVolumeObj,
            ScrollingFactorObj, SectionLenChangeObj, SpeedObj, StopObj, TextObj, WavObj,
        },
    },
    parse::{
        ParseOutput, ParseWarning, ParseWarningWithRange,
        check_playing::{PlayingCheckOutput, PlayingError, PlayingWarning},
        prompt::{
            AlwaysUseNewer, AlwaysUseOlder, AlwaysWarnAndUseNewer, AlwaysWarnAndUseOlder,
            DefDuplication, DuplicationWorkaround, Prompter,
        },
        token_processor::{common_preset, minor_preset, pedantic_preset},
        validity::{ValidityCheckOutput, ValidityInvalid, ValidityMissing},
    },
    parse_bms, parse_bms_with_preset,
    rng::{Rng, RngMock},
};

// Re-export related members when `rand` feature is enabled
#[cfg(feature = "rand")]
pub use super::rng::RandRng;

// Re-export minor command types when feature is enabled
#[cfg(feature = "minor-command")]
pub use super::{
    command::minor_command::{
        ExWavFrequency, ExWavPan, ExWavVolume, ExtChrEvent, StpEvent, SwBgaEvent, WavCmdEvent,
        WavCmdParam,
    },
    model::{
        def::ExWavDef,
        obj::{BgaArgbObj, BgaKeyboundObj, BgaOpacityObj, OptionObj, SeekObj},
    },
};
