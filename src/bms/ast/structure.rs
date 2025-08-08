//! The structure definition of the AST.
use std::collections::HashMap;

use num::BigUint;

use crate::bms::lex::token::Token;

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
    Token(&'a Token<'a>),
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
    /// The value of the If branch.
    pub value: BigUint,
    /// The tokens of the If branch.
    pub tokens: Vec<Unit<'a>>,
}

/// The define of a Case/Def branch in a Switch block.
/// Note: Def can appear in any position. If there is no other Case branch activated, Def will be activated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseBranch<'a> {
    /// The value of the Case branch.
    pub value: CaseBranchValue,
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