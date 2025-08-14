//! Prelude module for the BMS crate.
//!
//! This module re-exports all public types from the BMS module for convenient access.
//! You can use `use bms_rs::bms::prelude::*;` to import all BMS types at once.

// Re-export main types from bms module
pub use super::{BmsOutput, BmsWarning, Decimal, parse_bms};

// Re-export from command module
pub use super::command::{JudgeLevel, LnMode, LnType, ObjId, PlayerMode, PoorMode, Volume};

// Re-export from command submodules
pub use super::command::{
    channel::{
        Channel, Key, ModeKeyChannel, NoteKind, PlayerSide, convert_key_channel_between,
        read_channel_beat,
    },
    graphics::{Argb, PixelPoint, PixelSize, Rgb},
    time::{ObjTime, Track},
};

// Re-export from lex module
pub use super::lex::{BmsLexOutput, LexWarning, parse_lex_tokens};

// Re-export from parse module
pub use super::parse::{BmsParseOutput, BmsParseTokenIter, ParseWarning, ParseWarningWithPos};

// Re-export from parse submodules
pub use super::parse::{
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
        DuplicationWorkaround, PromptHandler, PromptingDuplication,
    },
    random::{
        ControlFlowRule,
        rng::{RandRng, Rng, RngMock},
    },
};

// Re-export from lex submodules
pub use super::lex::token::TokenWithPos;

// Re-export minor command types when feature is enabled
#[cfg(feature = "minor-command")]
pub use super::command::minor_command::{
    ExWavFrequency, ExWavPan, ExWavVolume, ExtChrEvent, StpEvent, SwBgaEvent, WavCmdEvent,
    WavCmdParam,
};

// Re-export ExWavDef when feature is enabled
#[cfg(feature = "minor-command")]
pub use super::parse::model::{
    def::ExWavDef,
    obj::{BgaArgbObj, BgaKeyboundObj, BgaOpacityObj, ExtendedMessageObj, OptionObj, SeekObj},
};
