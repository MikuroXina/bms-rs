//! This module introduces struct [`BmsSourceRepresentation`], which manages configurations of the representation format of BMS source.

use crate::bms::prelude::*;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// This aggregate manages configurations of the representation format of BMS source.
pub struct BmsSourceRepresentation {
    /// The LN notation type of the score.
    pub ln_type: LnType,
    /// LN Mode. Defines the long note mode for this chart.
    /// - 1: LN (Long Note)
    /// - 2: CN (Charge Note)
    /// - 3: HCN (Hell Charge Note)
    pub ln_mode: LnMode,
    /// Character set used in the BMS source originally (before UTF-8 conversion for this crate).
    pub charset: Option<String>,
    /// Raw lines that starts with `'#'`.
    pub raw_command_lines: Vec<String>,
    /// Lines that not starts with `'#'`.
    pub non_command_lines: Vec<String>,
    /// Whether the object ids are case-sensitive. It corresponds to `#BASE 62` extension command.
    pub case_sensitive_obj_id: bool,
}
