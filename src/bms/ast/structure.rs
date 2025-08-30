//! The structure of the AST.

use std::collections::BTreeMap;

use num::BigUint;

use crate::bms::{command::mixin::SourceRangeMixin, lex::token::TokenWithRange};

/// An unit of AST which represents individual scoped commands of BMS source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unit<'a> {
    /// A token that is not a control flow token.
    TokenWithRange(&'a TokenWithRange<'a>),
    /// A Random block. Can contain multiple If blocks.
    RandomBlock {
        /// The value of the Random block.
        value: SourceRangeMixin<BlockValue>,
        /// The If blocks in the Random block.
        if_blocks: Vec<IfBlock<'a>>,
        /// The ENDRANDOM position in the BMS source.
        end_random: SourceRangeMixin<()>,
    },
    /// A Switch block.
    /// Like C++ Programming Language, Switch block can contain multiple Case branches, and a Def branch.
    /// If there is no other Case branch activated, Def branch will be activated.
    /// When executing, the tokens, from the activated branch, to Skip/EndSwitch, will be executed.
    SwitchBlock {
        /// The value of the Switch block.
        value: SourceRangeMixin<BlockValue>,
        /// The Case branches in the Switch block.
        cases: Vec<CaseBranch<'a>>,
        /// The ENDSW position in the BMS source.
        end_sw: SourceRangeMixin<()>,
    },
}

/// The value of a Random/Switch block.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlockValue {
    /// For Random/Switch, value ranges in [1, max].
    /// IfBranch value must ranges in [1, max].
    Random {
        /// The maximum value of the Random/Switch block.
        max: BigUint,
    },
    /// For SetRandom/SetSwitch.
    /// IfBranch value has no limit.
    Set {
        /// The set value of the Random/Switch block.
        value: BigUint,
    },
}

/// The If block of a Random block. Should contain If/EndIf, can contain ElseIf/Else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfBlock<'a> {
    /// The branches of the If block.
    pub branches: BTreeMap<BigUint, SourceRangeMixin<Vec<Unit<'a>>>>,
    /// The ENDIF position in the BMS source for this IfBlock.
    /// If there is no matching `#ENDIF`, this falls back to the position of the last token
    /// inside the If block (or the current peek position when the block is empty).
    pub end_if: SourceRangeMixin<()>,
}

/// The define of a Case/Def branch in a Switch block.
/// Note: Def can appear in any position. If there is no other Case branch activated, Def will be activated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseBranch<'a> {
    /// The value of the Case/Def branch.
    pub value: SourceRangeMixin<CaseBranchValue>,
    /// The units in the Case/Def branch.
    pub units: Vec<Unit<'a>>,
}

/// The type note of a Case/Def branch.
/// Note: Def can appear in any position. If there is no other Case branch activated, Def will be activated.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CaseBranchValue {
    /// A Case branch.
    Case(BigUint),
    /// A Def branch.
    Def,
}
