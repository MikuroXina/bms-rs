//! Prelude module for the BMS crate.
//!
//! This module re-exports all public types from the BMS module for convenient access.
//! You can use `use bms_rs::bms::prelude::*;` to import all BMS types at once.

// Re-export types from bms module
pub use super::{
    BmsOutput, BmsWarning, Decimal,
    ast::{
        AstBuildOutput, AstParseOutput, AstRoot,
        rng::{RandRng, Rng, RngMock},
    },
    command::{
        JudgeLevel, LnMode, LnType, ObjId, PlayerMode, PoorMode, Volume,
        channel::{
            BeatModeMap, Channel, Key, KeyChannelMode, KeyChannelModeBeat,
            KeyChannelModeBeatNanasi, KeyChannelModeDscOctFp, KeyChannelModePms,
            KeyChannelModePmsBmeType, NoteKind, PlayerSide, convert_key_channel_between,
            read_channel_beat,
        },
        graphics::{Argb, PixelPoint, PixelSize, Rgb},
        mixin::{SourcePosMixin, SourcePosMixinExt},
        time::{ObjTime, Track},
    },
    lex::{
        BmsLexOutput, LexWarning, TokenRefStream, TokenStream,
        token::{Token, TokenWithPos},
    },
    parse::{
        BmsParseOutput, ParseWarning, ParseWarningWithPos,
        check_playing::{PlayingError, PlayingWarning},
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
    },
    parse_bms,
};

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
