//! Prelude module for the BMS crate.
//!
//! This module re-exports all public types from the BMS module for convenient access.
//! You can use `use bms_rs::bms::prelude::*;` to import all BMS types at once.

// Re-export types from bms module
pub use super::{
    BmsOutput, BmsWarning, Decimal,
    ast::{
        AstBuildOutput, AstBuildWarning, AstBuildWarningWithRange, AstParseOutput, AstParseWarning,
        AstParseWarningWithRange, AstRoot,
        rng::{Rng, RngMock},
        structure::{BlockValue, CaseBranch, CaseBranchValue, IfBlock, Unit},
    },
    command::{
        JudgeLevel, LnMode, LnType, ObjId, PlayerMode, PoorMode, Volume,
        channel::{
            Channel, Key, NoteKind, PlayerSide,
            converter::{
                KeyLayoutConvertLaneRandomShuffle, KeyLayoutConvertLaneRotateShuffle,
                KeyLayoutConvertMirror, KeyLayoutConverter,
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
    diagnostics::{SimpleSource, ToAriadne, emit_bms_warnings},
    lex::{
        LexOutput, LexWarning, TokenRefStream, TokenStream,
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
            DefDuplication, DuplicationWorkaround, PromptHandler,
        },
        validity::{ValidityCheckOutput, ValidityInvalid, ValidityMissing},
    },
    parse_bms_with_rng,
};

// Re-export related members when `rand` feature is enabled
#[cfg(feature = "rand")]
pub use super::{ast::rng::RandRng, parse_bms};

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
