//! Key mapping types for converting between channel IDs and key layouts.
//!
//! This module provides traits and implementations for mapping BMS channel IDs
//! to player-side key positions, supporting multiple key layouts (Beat, PMS, etc.)
//! and key conversion operations (mirror, shuffle, flip).

pub mod check;
pub mod converter;
pub mod mapper;

pub use check::check_bms_validity;
pub use converter::{
    KeyConverter, KeyMappingConvertFlip, KeyMappingConvertLaneRandomShuffle,
    KeyMappingConvertLaneRotateShuffle, KeyMappingConvertMirror, PlayerSideKeyConverter,
};
pub use mapper::{
    KeyLayoutBeat, KeyLayoutBeatNanasi, KeyLayoutDscOctFp, KeyLayoutMapper, KeyLayoutPms,
    KeyLayoutPmsBmeType, KeyMapping,
};
