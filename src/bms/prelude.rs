//! Prelude module for the BMS crate.
//!
//! This module re-exports all public types from the BMS module for convenient access.
//! You can use `use bms_rs::bms::prelude::*;` to import all BMS types at once.

// Re-export main types from bms module
pub use super::{BmsWarning, Decimal};

// Re-export from command module
pub use super::command::{
    Argb, ExtChrEvent, JudgeLevel, LnModeType, LnType, ObjId, PlayerMode, PoorMode, Rgb, Volume,
    WavCmdParam,
};

// Re-export from command submodules
pub use super::command::channel::{Channel, Key, NoteKind, PlayerSide};
pub use super::command::graphics::{PixelPoint, PixelSize};
pub use super::command::time::{ObjTime, Track};

// Re-export from lex module
pub use super::lex::{BmsLexOutput, LexWarning};

// Re-export from parse module
pub use super::parse::{BmsParseOutput, BmsParseTokenIter, ParseWarning};

// Re-export from parse submodules
pub use super::parse::check_playing::{PlayingError, PlayingWarning};
pub use super::parse::model::def::{AtBgaDef, BgaDef, Bmp, ExRankDef};
pub use super::parse::model::obj::{
    BgaLayer, BgaObj, BpmChangeObj, ExtendedMessageObj, Obj, ScrollingFactorObj,
    SectionLenChangeObj, SpeedObj, StopObj,
};
pub use super::parse::model::{Arrangers, Bms, Graphics, Header, Notes, Others, ScopeDefines};
pub use super::parse::prompt::{
    AlwaysUseNewer, AlwaysUseOlder, AlwaysWarn, DuplicationWorkaround, PromptHandler,
    PromptingDuplication,
};
pub use super::parse::random::ControlFlowRule;
pub use super::parse::random::rng::{RandRng, Rng, RngMock};

// Re-export from lex submodules
pub use super::lex::token::Token;

// Re-export minor command types when feature is enabled
#[cfg(feature = "minor-command")]
pub use super::command::minor_command::{
    ExWavFrequency, ExWavPan, ExWavVolume, StpEvent, SwBgaEvent, WavCmdEvent,
};

// Re-export ExWavDef when feature is enabled
#[cfg(feature = "minor-command")]
pub use super::parse::model::def::ExWavDef;
