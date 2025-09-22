//! Prelude module for the BMSON crate.
//!
//! This module re-exports all public types from the BMSON module for convenient access.
//! You can use `use bms_rs::bmson::prelude::*;` to import all BMSON types at once.
//!
//! This prelude also includes commonly used BMS types that are dependencies of BMSON types.

// Re-export main BMSON types
pub use super::{
    BarLine, Bga, BgaEvent, BgaHeader, BgaId, Bmson, BmsonInfo, KeyChannel, KeyEvent, MineChannel,
    MineEvent, Note, ScrollEvent, SoundChannel,
};

// Re-export event types
pub use super::{BpmEvent, StopEvent};

// Re-export conversion types and warnings
pub use super::bms_to_bmson::{BmsToBmsonOutput, BmsToBmsonWarning};

pub use super::bmson_to_bms::{BmsonToBmsOutput, BmsonToBmsWarning};

// Re-export utility types
pub use super::{
    fin_f64::{FinF64, TryFromFloatError},
    pulse::{PulseConverter, PulseNumber},
};

// Re-export parsing functions and types
pub use super::{BmsonParseError, BmsonParseOutput, parse_bmson};

// Re-export default functions
pub use super::{default_mode_hint, default_percentage, default_resolution};

// Re-export BMS types that are dependencies of BMSON types
pub use crate::bms::command::LnMode;
