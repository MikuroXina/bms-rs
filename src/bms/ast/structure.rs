//! The structure definition of the AST.
use std::collections::HashMap;

use num::BigUint;
use thiserror::Error;

use crate::bms::{command::PositionWrapper, lex::token::TokenContent};
use crate::command::PositionWrapperExt;

/// The root of the AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstRoot<'a> {
    /// The units of the AST.
    pub units: Vec<Unit<'a>>,
}

/// An unit of AST which represents individual scoped commands of BMS source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unit<'a> {
    /// A token that is not a control flow token.
    Token(&'a PositionWrapper<TokenContent<'a>>),
    /// A Random block. Can contain multiple If blocks.
    RandomBlock {
        /// The value of the Random block.
        value: BlockValue,
        /// The If blocks of the Random block.
        if_blocks: Vec<IfBlock<'a>>,
    },
    /// A Switch block.
    /// Like C++ Programming Language, Switch block can contain multiple Case branches, and a Def branch.
    /// If there is no other Case branch activated, Def branch will be activated.
    /// When executing, the tokens, from the activated branch, to Skip/EndSwitch, will be executed.
    SwitchBlock {
        /// The value of the Switch block.
        value: BlockValue,
        /// The Case branches of the Switch block.
        cases: Vec<CaseBranch<'a>>,
    },
}

/// The value of a Random/Switch block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockValue {
    /// For Random/Switch, value ranges in [1, max].
    /// IfBranch value must ranges in [1, max].
    Random {
        /// The max value of the Random block.
        max: BigUint,
    },
    /// For SetRandom/SetSwitch.
    /// IfBranch value has no limit.
    Set {
        /// The value of the SetRandom/SetSwitch.
        value: BigUint,
    },
}

/// The If block of a Random block. Should contain If/EndIf, can contain ElseIf/Else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfBlock<'a> {
    /// The branches of the If block.
    pub branches: HashMap<BigUint, IfBranch<'a>>,
}

/// The If branch of a If block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfBranch<'a> {
    /// The value of the If branch, with position info.
    pub value: PositionWrapper<BigUint>,
    /// The tokens of the If branch.
    pub tokens: Vec<Unit<'a>>,
}

/// The define of a Case/Def branch in a Switch block.
/// Note: Def can appear in any position. If there is no other Case branch activated, Def will be activated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseBranch<'a> {
    /// The value of the Case branch, with position info.
    pub value: PositionWrapper<CaseBranchValue>,
    /// The tokens of the Case branch.
    pub tokens: Vec<Unit<'a>>,
}

/// The type note of a Case/Def branch.
/// Note: Def can appear in any position. If there is no other Case branch activated, Def will be activated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseBranchValue {
    /// The value of the Case branch.
    Case(BigUint),
    /// The value of the Def branch.
    Def,
}

/// Control flow parsing errors and warnings.
///
/// This enum defines all possible errors that can occur during BMS control flow parsing.
/// Each variant represents a specific type of control flow violation or malformed construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AstBuildWarningType {
    /// An `#ENDIF` token was encountered without a corresponding `#IF` token.
    #[error("unmatched end if")]
    UnmatchedEndIf,
    /// An `#ENDRANDOM` token was encountered without a corresponding `#RANDOM` token.
    #[error("unmatched end random")]
    UnmatchedEndRandom,
    /// An `#ENDSWITCH` token was encountered without a corresponding `#SWITCH` token.
    #[error("unmatched end switch")]
    UnmatchedEndSwitch,
    /// An `#ELSEIF` token was encountered without a corresponding `#IF` token.
    #[error("unmatched else if")]
    UnmatchedElseIf,
    /// An `#ELSE` token was encountered without a corresponding `#IF` token.
    #[error("unmatched else")]
    UnmatchedElse,
    /// A duplicate `#IF` branch value was found in a random block.
    #[error("duplicate if branch value in random block")]
    RandomDuplicateIfBranchValue,
    /// An `#IF` branch value exceeds the maximum value of its random block.
    #[error("if branch value out of range in random block")]
    RandomIfBranchValueOutOfRange,
    /// Tokens were found between `#RANDOM` and `#IF` that should not be there.
    #[error("unmatched token in random block, e.g. Tokens between Random and If.")]
    UnmatchedTokenInRandomBlock,
    /// A duplicate `#CASE` value was found in a switch block.
    #[error("duplicate case value in switch block")]
    SwitchDuplicateCaseValue,
    /// A `#CASE` value exceeds the maximum value of its switch block.
    #[error("case value out of range in switch block")]
    SwitchCaseValueOutOfRange,
    /// Multiple `#DEF` branches were found in the same switch block.
    #[error("duplicate def branch in switch block")]
    SwitchDuplicateDef,
    /// A `#SKIP` token was encountered outside of a switch block.
    #[error("unmatched skip")]
    UnmatchedSkip,
    /// A `#CASE` token was encountered outside of a switch block.
    #[error("unmatched case")]
    UnmatchedCase,
    /// A `#DEF` token was encountered outside of a switch block.
    #[error("unmatched def")]
    UnmatchedDef,
}

impl PositionWrapperExt for AstBuildWarningType {}
impl PositionWrapperExt for CaseBranchValue {}
// `AstBuildWarning` 类型别名已删除，请直接使用 `PositionWrapper<AstBuildWarningType>`。

/// Control flow parsing warnings emitted during AST execution (parse phase).
///
/// These warnings are produced when evaluating the AST, e.g. validating value ranges
/// for `#RANDOM/#SWITCH` against their branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AstParseWarningType {
    /// An `#IF` branch value exceeds the maximum of its `#RANDOM` block.
    #[error("if branch value out of range in random block")]
    RandomIfBranchValueOutOfRange,
    /// A `#CASE` value exceeds the maximum of its `#SWITCH` block.
    #[error("case value out of range in switch block")]
    SwitchCaseValueOutOfRange,
}

impl PositionWrapperExt for AstParseWarningType {}
// `AstParseWarning` 类型别名已删除，请直接使用 `PositionWrapper<AstParseWarningType>`。
