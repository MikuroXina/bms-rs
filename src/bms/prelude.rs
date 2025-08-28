//! Prelude module for the BMS crate.
//!
//! This module re-exports all public types from the BMS module for convenient access.
//! You can use `use bms_rs::bms::prelude::*;` to import all BMS types at once.

// Re-export types from bms module
pub use super::{
    BmsOutput, BmsWarning, Decimal,
    ast::{
        AstBuildOutput, AstParseOutput, AstRoot,
        rng::{Rng, RngMock},
    },
    command::{
        JudgeLevel, LnMode, LnType, ObjId, PlayerMode, PoorMode, Volume,
        channel::{
            Channel, Key, KeyMapping, NoteKind, PlayerSide,
            converter::{
                KeyLayoutConvertLaneRandomShuffle, KeyLayoutConvertLaneRotateShuffle,
                KeyLayoutConvertMirror, KeyLayoutConverter,
            },
            mapper::{
                KeyLayoutBeat, KeyLayoutBeatNanasi, KeyLayoutDscOctFp, KeyLayoutMapper,
                KeyLayoutPms, KeyLayoutPmsBmeType,
            },
            read_channel_beat,
        },
        graphics::{Argb, PixelPoint, PixelSize, Rgb},
        mixin::{SourcePosMixin, SourcePosMixinExt},
        time::{ObjTime, Track},
    },
    lex::{
        LexOutput, LexWarning, TokenRefStream, TokenStream,
        token::{Token, TokenWithPos},
    },
    parse::{
        ParseOutput, ParseWarning, ParseWarningWithPos,
        check_playing::{PlayingCheckOutput, PlayingError, PlayingWarning},
        model::{
            Arrangers, Bms, Graphics, Header, Notes, Others, ScopeDefines,
            def::{AtBgaDef, BgaDef, Bmp, ExRankDef},
            obj::{
                BgaLayer, BgaObj, BgmVolumeObj, BpmChangeObj, JudgeObj, KeyVolumeObj, Obj,
                ScrollingFactorObj, SectionLenChangeObj, SpeedObj, StopObj, TextObj,
            },
        },
        prompt::{
            AlwaysUseNewer, AlwaysUseOlder, AlwaysWarnAndUseNewer, AlwaysWarnAndUseOlder,
            DefDuplication, DuplicationWorkaround, PromptHandler,
        },
        validity::{
            ValidityCheckOutput, ValidityEmpty, ValidityInvalid, ValidityMapping, ValidityMissing,
        },
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
    parse::model::{
        def::ExWavDef,
        obj::{BgaArgbObj, BgaKeyboundObj, BgaOpacityObj, OptionObj, SeekObj},
    },
};
