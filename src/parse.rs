//! Parsing modules for BMS and BMSON formats.
//!
//! This module contains parsers for different BMS-related formats.
//! Currently supported:
//!
//! - `bms`: The original BMS format parser (.bms/.bme/.bml/.pms)
//! - `bmson`: The bmson JSON format

pub mod bms;
pub mod bmson;
