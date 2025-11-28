//! Random model: `IfBlock` and unified `Random` structure.
//!
//! This module defines a lightweight model to build `#RANDOM`/`#SETRANDOM` blocks
//! using regular BMS tokens. Branch entries accept tokens with any lifetime
//! (`Token<'a>`), so you can construct random blocks from borrowed strings
//! without requiring `'static` data.
pub mod activate;
pub mod construct;
pub mod into_tokens;

use std::borrow::Cow;
use std::ops::{Index, IndexMut};

use num::BigUint;

use crate::bms::command::mixin::{MaybeWithRange, SourceRangeMixin};
use crate::bms::lex::token::{Token, TokenWithRange};

/// A non-control-flow token view used inside branch contents.
///
/// Wraps `Cow<Token>` optionally with range metadata using `MaybeWithRange`.
/// This preserves source positions when available and avoids cloning when
/// borrowing is sufficient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonControlToken<'a>(MaybeWithRange<Cow<'a, Token<'a>>>);

impl<'a> NonControlToken<'a> {
    /// Attempts to create an owned non-control token from a plain `Token`.
    ///
    /// Returns `Err(Token)` if the token is a control-flow header.
    pub fn try_from_token<C>(c: C) -> Result<Self, Cow<'a, Token<'a>>>
    where
        C: Into<Cow<'a, Token<'a>>>,
    {
        let cow = c.into();
        if cow.is_control_flow_token() {
            Err(cow)
        } else {
            Ok(Self(MaybeWithRange::plain(cow)))
        }
    }

    /// Attempts to create a borrowed non-control token from a `TokenWithRange`.
    ///
    /// Returns `Err(Token)` if the token is a control-flow header.
    /// Attempts to create a borrowed non-control token from a `TokenWithRange`.
    ///
    /// Returns `Err(Token)` if the token is a control-flow header.
    pub fn try_from_token_with_range<C>(c: C) -> Result<Self, Cow<'a, TokenWithRange<'a>>>
    where
        C: Into<Cow<'a, TokenWithRange<'a>>>,
    {
        let cow = c.into();
        if cow.as_ref().content().is_control_flow_token() {
            Err(cow)
        } else {
            match cow {
                Cow::Borrowed(wr) => Ok(Self(MaybeWithRange::wrapped(SourceRangeMixin::new(
                    Cow::Borrowed(wr.content()),
                    wr.range().clone(),
                )))),
                Cow::Owned(wr) => {
                    let (tok, range) = wr.into();
                    Ok(Self(MaybeWithRange::wrapped(SourceRangeMixin::new(
                        Cow::Owned(tok),
                        range,
                    ))))
                }
            }
        }
    }
}

/// A unit of branch content that can represent nested control flow or plain non-control tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenUnit<'a> {
    /// Nested random block content.
    Random(Random<'a>),
    /// Nested switch block content.
    Switch(Switch<'a>),
    /// Plain non-control-flow tokens for branch content.
    Tokens(Vec<NonControlToken<'a>>),
}

impl<'a> From<Random<'a>> for TokenUnit<'a> {
    fn from(value: Random<'a>) -> Self {
        TokenUnit::Random(value)
    }
}

impl<'a> From<Switch<'a>> for TokenUnit<'a> {
    fn from(value: Switch<'a>) -> Self {
        TokenUnit::Switch(value)
    }
}

impl<'a> From<Vec<NonControlToken<'a>>> for TokenUnit<'a> {
    fn from(value: Vec<NonControlToken<'a>>) -> Self {
        TokenUnit::Tokens(value)
    }
}

/// Indicates whether the random block generates a value (`#RANDOM`) or uses a set value (`#SETRANDOM`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlFlowValue {
    /// Use a fixed value (emits `#SETRANDOM <value>`).
    Set(BigUint),
    /// Generate a random value with the given maximum (emits `#RANDOM <max>`).
    GenMax(BigUint),
}

/// One branch node in an if-chain inside a random block.
#[derive(Debug, Clone, PartialEq, Eq)]
enum IfChainEntry<'a> {
    /// An `#ELSEIF <cond>` branch with its units and a pointer to the next entry.
    ElseIf {
        /// Condition value for `#ELSEIF <cond>`.
        cond: MaybeWithRange<BigUint>,
        /// Content units for this branch.
        units: Vec<TokenUnit<'a>>,
        /// Pointer to the next chain entry.
        next: Box<IfChainEntry<'a>>,
    },
    /// An `#ELSE` branch with its units. This is always terminal.
    Else {
        /// Content units for this branch.
        units: Vec<TokenUnit<'a>>,
    },
    /// Terminator for an if-chain.
    EndIf,
}

impl<'a> IfChainEntry<'a> {
    fn append_to_tail(&mut self, entry: Self) {
        match self {
            IfChainEntry::EndIf => {
                *self = entry;
            }
            IfChainEntry::ElseIf { next, .. } => {
                next.append_to_tail(entry);
            }
            IfChainEntry::Else { .. } => {
                // Already terminal; do not append beyond ELSE.
            }
        }
    }

    fn units_at(&self, chain_index: usize) -> Option<&[TokenUnit<'a>]> {
        // chain_index refers to entries after the head: 1 => first else-if/else
        let mut idx = chain_index;
        let mut cur = self;
        while idx > 0 {
            match cur {
                IfChainEntry::ElseIf { next, .. } => {
                    idx -= 1;
                    cur = next.as_ref();
                }
                IfChainEntry::Else { .. } => return None,
                IfChainEntry::EndIf => return None,
            }
        }
        match cur {
            IfChainEntry::ElseIf { units, .. } | IfChainEntry::Else { units } => Some(units),
            IfChainEntry::EndIf => None,
        }
    }

    fn set_units_at<U>(&mut self, chain_index: usize, new_units: U) -> Option<Vec<TokenUnit<'a>>>
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        let mut idx = chain_index;
        let mut cur = self;
        loop {
            match cur {
                IfChainEntry::ElseIf { units, next, .. } => {
                    if idx == 0 {
                        let old = std::mem::replace(units, new_units.into_iter().collect());
                        return Some(old);
                    } else {
                        idx -= 1;
                        cur = next.as_mut();
                    }
                }
                IfChainEntry::Else { units } => {
                    if idx == 0 {
                        let old = std::mem::replace(units, new_units.into_iter().collect());
                        return Some(old);
                    } else {
                        return None;
                    }
                }
                IfChainEntry::EndIf => return None,
            }
        }
    }
}

/// If-chain used within a random block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfBlock<'a> {
    condition: MaybeWithRange<BigUint>,
    head_units: Vec<TokenUnit<'a>>, // units for the initial `#IF` branch
    chain: IfChainEntry<'a>,        // subsequent `ELSEIF`/`ELSE` nodes
}

impl<'a> IfBlock<'a> {
    /// Create a new if-chain with units in the first `#IF` entry.
    pub fn new_if<C, U>(cond: C, units: U) -> Self
    where
        C: Into<MaybeWithRange<BigUint>>,
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        Self {
            condition: cond.into(),
            head_units: units.into_iter().collect(),
            chain: IfChainEntry::EndIf,
        }
    }

    /// Add an `#ELSEIF <cond>` entry with units.
    pub fn or_else_if<C, U>(mut self, cond: C, units: U) -> Self
    where
        C: Into<MaybeWithRange<BigUint>>,
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        let entry = IfChainEntry::ElseIf {
            cond: cond.into(),
            units: units.into_iter().collect(),
            next: Box::new(IfChainEntry::EndIf),
        };
        self.chain.append_to_tail(entry);
        self
    }

    /// Add an `#ELSE` entry with units.
    pub fn or_else<U>(mut self, units: U) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        let entry = IfChainEntry::Else {
            units: units.into_iter().collect(),
        };
        self.chain.append_to_tail(entry);
        self
    }

    /// Returns a view of the head `#IF` units.
    #[must_use]
    pub fn head_units(&self) -> &[TokenUnit<'a>] {
        &self.head_units
    }

    /// Replace units for the branch at `index`. `index=0` refers to the head `#IF` branch;
    /// `index>=1` refers to the subsequent `ELSEIF`/`ELSE` nodes.
    /// Returns previous units when the index exists.
    pub fn set_units_at<U>(&mut self, index: usize, new_units: U) -> Option<Vec<TokenUnit<'a>>>
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        if index == 0 {
            let old = std::mem::replace(&mut self.head_units, new_units.into_iter().collect());
            Some(old)
        } else {
            self.chain.set_units_at(index - 1, new_units)
        }
    }

    /// Returns a view of the units for the branch at `index`.
    #[must_use]
    pub fn units_at(&self, index: usize) -> Option<&[TokenUnit<'a>]> {
        if index == 0 {
            Some(&self.head_units)
        } else {
            self.chain.units_at(index - 1)
        }
    }

    /// Set a new condition for the head `#IF` entry, returning the previous value.
    pub fn set_condition<C>(&mut self, new_condition: C) -> BigUint
    where
        C: Into<MaybeWithRange<BigUint>>,
    {
        let prev = std::mem::replace(&mut self.condition, new_condition.into());
        prev.into_content()
    }

    /// Returns the number of branches in this if-chain (including head `#IF`).
    #[must_use]
    pub fn len(&self) -> usize {
        let mut count = 1; // head if
        let mut cur = &self.chain;
        loop {
            match cur {
                IfChainEntry::ElseIf { next, .. } => {
                    count += 1;
                    cur = next.as_ref();
                }
                IfChainEntry::Else { .. } => {
                    count += 1;
                    break;
                }
                IfChainEntry::EndIf => break,
            }
        }
        count
    }

    /// An if-chain always has a head `#IF` branch, so it is never empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }
}

/// A random block (`#RANDOM` or `#SETRANDOM`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Random<'a> {
    value: MaybeWithRange<ControlFlowValue>,
    branches: Vec<IfBlock<'a>>,
}

impl<'a> Random<'a> {
    /// Create an empty random block (`#RANDOM` or `#SETRANDOM`).
    #[must_use]
    pub fn new<V>(value: V) -> Self
    where
        V: Into<MaybeWithRange<ControlFlowValue>>,
    {
        Self {
            value: value.into(),
            branches: Vec::new(),
        }
    }

    /// Append an `IfBlock` branch for chained construction.
    #[must_use]
    pub fn if_block(mut self, branch: IfBlock<'a>) -> Self {
        self.branches.push(branch);
        self
    }

    /// Number of branches.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.branches.len()
    }

    /// Returns true if there are no branches in this random block.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.branches.is_empty()
    }

    /// Get branch by index.
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&IfBlock<'a>> {
        self.branches.get(index)
    }

    /// Get mutable branch by index.
    pub fn at_mut(&mut self, index: usize) -> Option<&mut IfBlock<'a>> {
        self.branches.get_mut(index)
    }
}

impl<'a> IntoIterator for Random<'a> {
    type Item = IfBlock<'a>;
    type IntoIter = std::vec::IntoIter<IfBlock<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.branches.into_iter()
    }
}

impl<'b, 'a> IntoIterator for &'b Random<'a> {
    type Item = &'b IfBlock<'a>;
    type IntoIter = std::slice::Iter<'b, IfBlock<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.branches.iter()
    }
}

impl<'a> Index<usize> for Random<'a> {
    type Output = IfBlock<'a>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.branches[index]
    }
}

impl<'a> IndexMut<usize> for Random<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.branches[index]
    }
}

/// One case in a switch block. `condition = None` means `#DEF`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CaseEntry<'a> {
    condition: Option<MaybeWithRange<BigUint>>,
    units: Vec<TokenUnit<'a>>, // case content can be nested control flow or tokens
    skip: bool,                // whether to emit `#SKIP` after tokens
}

impl<'a> CaseEntry<'a> {
    /// Create a case entry with condition (units only).
    pub fn new<C, U>(cond: C, units: U) -> Self
    where
        C: Into<MaybeWithRange<BigUint>>,
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        Self {
            condition: Some(cond.into()),
            units: units.into_iter().collect(),
            skip: true,
        }
    }

    /// Create a default entry (`#DEF`).
    pub fn default<U>(units: U) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        Self {
            condition: None,
            units: units.into_iter().collect(),
            skip: true,
        }
    }

    /// Set whether to emit `#SKIP` after tokens (default: true).
    pub const fn set_skip(&mut self, skip: bool) {
        self.skip = skip;
    }
}

/// A switch block (`#SWITCH` or `#SETSWITCH`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switch<'a> {
    value: MaybeWithRange<ControlFlowValue>,
    cases: Vec<CaseEntry<'a>>,
}

impl<'a> Switch<'a> {
    /// Start building a `Switch` with a control-flow value.
    /// This returns an empty `Switch` to be populated via builder-style methods.
    #[must_use]
    pub fn new<V>(value: V) -> Self
    where
        V: Into<MaybeWithRange<ControlFlowValue>>,
    {
        Self {
            value: value.into(),
            cases: Vec::new(),
        }
    }

    /// Add a `#CASE <cond>` branch and emit `#SKIP` after tokens.
    pub fn case_with_skip<C, U>(mut self, cond: C, units: U) -> Self
    where
        C: Into<MaybeWithRange<BigUint>>,
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        let mut entry = CaseEntry::new(cond.into(), units);
        entry.set_skip(true);
        self.cases.push(entry);
        self
    }

    /// Add a `#CASE <cond>` branch and do not emit `#SKIP` after tokens.
    pub fn case_no_skip<C, U>(mut self, cond: C, units: U) -> Self
    where
        C: Into<MaybeWithRange<BigUint>>,
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        let mut entry = CaseEntry::new(cond.into(), units);
        entry.set_skip(false);
        self.cases.push(entry);
        self
    }

    /// Add a `#DEF` default branch. `skip` defaults to true.
    pub fn def<U>(mut self, units: U) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        self.cases.push(CaseEntry::default(units));
        self
    }

    /// Add a `#DEF` default branch with explicit `skip` control.
    pub fn def_with_skip<U>(mut self, units: U, skip: bool) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        let mut entry = CaseEntry::default(units);
        entry.set_skip(skip);
        self.cases.push(entry);
        self
    }

    /// Finalize and return the built `Switch`. This is a no-op for chaining symmetry.
    #[must_use]
    pub const fn build(self) -> Self {
        self
    }

    /// Number of cases.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.cases.len()
    }

    /// Returns true if there are no cases in this switch block.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cases.is_empty()
    }
}
impl<'a> TokenUnit<'a> {
    /// Constructs a `Tokens` unit from an iterator of owned tokens.
    ///
    /// Control-flow headers are filtered out; only non-control tokens remain.
    #[must_use]
    pub fn from_tokens<T>(tokens: T) -> Self
    where
        T: IntoIterator<Item = Token<'a>>,
    {
        let v = tokens
            .into_iter()
            .map(|t| NonControlToken::try_from_token(Cow::Owned(t)))
            .flat_map(Result::ok)
            .collect();
        Self::Tokens(v)
    }
}
