//! Random model: `If` and unified `Random` structure.
//!
//! This module defines a lightweight model to build `#RANDOM`/`#SETRANDOM` blocks
//! using regular BMS tokens. Branch entries accept tokens with any lifetime
//! (`Token<'a>`), so you can construct random blocks from borrowed strings
//! without requiring `'static` data.

use std::ops::{Index, IndexMut};

use num::BigUint;

use crate::bms::lex::token::Token;

/// A token guaranteed to be non-control-flow.
///
/// Wraps a regular `Token` but ensures it is not any of the control flow headers
/// such as `#RANDOM`, `#IF`, `#SWITCH`, etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonControlToken<'a>(Token<'a>);

impl<'a> NonControlToken<'a> {
    /// Attempt to create a `NonControlToken` from a `Token`.
    /// Returns `None` if the token is a control-flow token.
    pub fn try_from_token(token: Token<'a>) -> Result<Self, Token<'a>> {
        if token.is_control_flow_token() {
            Err(token)
        } else {
            Ok(Self(token))
        }
    }

    /// Borrow the inner `Token`.
    #[must_use]
    pub const fn as_token(&self) -> &Token<'a> {
        &self.0
    }

    /// Consume and return the inner `Token`.
    #[must_use]
    pub fn into_token(self) -> Token<'a> {
        self.0
    }
}

impl<'a> From<NonControlToken<'a>> for Token<'a> {
    fn from(value: NonControlToken<'a>) -> Self {
        value.0
    }
}

impl<'a> TryFrom<Token<'a>> for NonControlToken<'a> {
    type Error = Token<'a>;

    fn try_from(value: Token<'a>) -> Result<Self, Self::Error> {
        Self::try_from_token(value)
    }
}

/// Alias preferred by external APIs/tests.
pub type NonControlFlowToken<'a> = NonControlToken<'a>;

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

impl<'a> TokenUnit<'a> {
    /// Create a `Tokens` unit from an iterator of raw tokens, filtering out control-flow ones.
    #[must_use]
    pub fn from_tokens<T>(tokens: T) -> Self
    where
        T: IntoIterator<Item = Token<'a>>,
    {
        let v = tokens
            .into_iter()
            .map(NonControlToken::try_from_token)
            .flat_map(Result::ok)
            .collect();
        Self::Tokens(v)
    }

    /// Convert this unit into lex tokens.
    #[must_use]
    pub fn into_tokens(self) -> Vec<Token<'a>> {
        match self {
            TokenUnit::Random(r) => r.into_tokens(),
            TokenUnit::Switch(s) => s.into_tokens(),
            TokenUnit::Tokens(v) => v.into_iter().map(Token::from).collect(),
        }
    }
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

/// One branch in an if-chain inside a random block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfChainEntry<'a> {
    condition: Option<BigUint>,
    units: Vec<TokenUnit<'a>>, // branch content can be nested control flow or tokens
}

impl<'a> IfChainEntry<'a> {
    /// Create entry with explicit units (supports nested Random/Switch).
    fn new<U>(condition: Option<BigUint>, units: U) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        Self {
            condition,
            units: units.into_iter().collect(),
        }
    }

    /// Returns the condition if present (None for `else`).
    #[must_use]
    pub const fn condition(&self) -> Option<&BigUint> {
        self.condition.as_ref()
    }

    /// Returns a view of the branch content units.
    #[must_use]
    pub fn units(&self) -> &[TokenUnit<'a>] {
        &self.units
    }

    /// Set a new condition for this entry.
    /// Returns the previous condition when this entry had a condition,
    /// or None if this is an `else` entry (no change is applied).
    pub fn set_condition(&mut self, new_condition: BigUint) -> Option<BigUint> {
        self.condition
            .as_mut()
            .map(|cond| std::mem::replace(cond, new_condition))
    }

    /// Replace units for this entry.
    /// Returns the previous units.
    pub fn set_units<U>(&mut self, new_units: U) -> Vec<TokenUnit<'a>>
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        let mut filtered: Vec<TokenUnit<'a>> = new_units.into_iter().collect();
        std::mem::swap(&mut filtered, &mut self.units);
        filtered
    }
}

/// If-chain used within a random block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct If<'a> {
    entries: Vec<IfChainEntry<'a>>,
}

impl<'a> If<'a> {
    /// Create a new if-chain with units in the first `if` entry.
    pub fn new<U>(cond: BigUint, units: U) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        Self {
            entries: vec![IfChainEntry::new(Some(cond), units)],
        }
    }

    /// Add an `else if` entry with units.
    pub fn or_else_if<U>(mut self, cond: BigUint, units: U) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        self.entries.push(IfChainEntry::new(Some(cond), units));
        self
    }

    /// Add an `else` entry with units.
    pub fn or_else<U>(mut self, units: U) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        self.entries.push(IfChainEntry::new(None, units));
        self
    }

    /// Get an entry by index.
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&IfChainEntry<'a>> {
        self.entries.get(index)
    }

    /// Get a mutable entry by index.
    pub fn at_mut(&mut self, index: usize) -> Option<&mut IfChainEntry<'a>> {
        self.entries.get_mut(index)
    }
}

impl<'a> Index<usize> for If<'a> {
    type Output = IfChainEntry<'a>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl<'a> IndexMut<usize> for If<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

/// A random block (`#RANDOM` or `#SETRANDOM`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Random<'a> {
    value: ControlFlowValue,
    branches: Vec<If<'a>>,
}

impl<'a> Random<'a> {
    /// Create a random block (`#RANDOM` or `#SETRANDOM`) with unified constructor.
    pub fn new<T>(value: ControlFlowValue, branches: T) -> Self
    where
        T: IntoIterator<Item = If<'a>>,
    {
        Self {
            value,
            branches: branches.into_iter().collect(),
        }
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
    pub fn at(&self, index: usize) -> Option<&If<'a>> {
        self.branches.get(index)
    }

    /// Get mutable branch by index.
    pub fn at_mut(&mut self, index: usize) -> Option<&mut If<'a>> {
        self.branches.get_mut(index)
    }

    /// Convert the model into lex tokens representing the random block.
    #[must_use]
    pub fn into_tokens(self) -> Vec<Token<'a>> {
        let mut out = Vec::new();
        match &self.value {
            ControlFlowValue::GenMax(max) => out.push(Token::Header {
                name: "RANDOM".into(),
                args: max.to_string().into(),
            }),
            ControlFlowValue::Set(val) => out.push(Token::Header {
                name: "SETRANDOM".into(),
                args: val.to_string().into(),
            }),
        }

        self.branches.into_iter().for_each(|branch| {
            out.extend(
                branch
                    .entries
                    .into_iter()
                    .enumerate()
                    .flat_map(|(i, entry)| {
                        let head = match (i == 0, entry.condition) {
                            (true, Some(cond)) => Token::Header {
                                name: "IF".into(),
                                args: cond.to_string().into(),
                            },
                            (false, Some(cond)) => Token::Header {
                                name: "ELSEIF".into(),
                                args: cond.to_string().into(),
                            },
                            (_, None) => Token::Header {
                                name: "ELSE".into(),
                                args: "".into(),
                            },
                        };

                        std::iter::once(head)
                            .chain(entry.units.into_iter().flat_map(TokenUnit::into_tokens))
                    })
                    .chain(std::iter::once(Token::Header {
                        name: "ENDIF".into(),
                        args: "".into(),
                    })),
            );
        });

        out.push(Token::Header {
            name: "ENDRANDOM".into(),
            args: "".into(),
        });

        out
    }
}

impl<'a> IntoIterator for Random<'a> {
    type Item = If<'a>;
    type IntoIter = std::vec::IntoIter<If<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.branches.into_iter()
    }
}

impl<'b, 'a> IntoIterator for &'b Random<'a> {
    type Item = &'b If<'a>;
    type IntoIter = std::slice::Iter<'b, If<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.branches.iter()
    }
}

impl<'a> Index<usize> for Random<'a> {
    type Output = If<'a>;

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
pub struct CaseEntry<'a> {
    condition: Option<BigUint>,
    units: Vec<TokenUnit<'a>>, // case content can be nested control flow or tokens
    skip: bool,                // whether to emit `#SKIP` after tokens
}

impl<'a> CaseEntry<'a> {
    /// Create a case entry with condition (units only).
    pub fn new<U>(cond: BigUint, units: U) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        Self {
            condition: Some(cond),
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

    // (removed token-based constructors; units-only API is enforced)

    /// Set whether to emit `#SKIP` after tokens (default: true).
    pub const fn set_skip(&mut self, skip: bool) {
        self.skip = skip;
    }

    /// Returns the condition if present (None for `default`).
    #[must_use]
    pub const fn condition(&self) -> Option<&BigUint> {
        self.condition.as_ref()
    }

    /// Returns a view of the non-control tokens contained in this case.
    #[must_use]
    pub fn units(&self) -> &[TokenUnit<'a>] {
        &self.units
    }

    // (removed token-based setter; use `set_units` instead)

    /// Replace units for this case.
    /// Returns the previous units.
    pub fn set_units<U>(&mut self, new_units: U) -> Vec<TokenUnit<'a>>
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        let mut filtered: Vec<TokenUnit<'a>> = new_units.into_iter().collect();
        std::mem::swap(&mut filtered, &mut self.units);
        filtered
    }
}

/// A switch block (`#SWITCH` or `#SETSWITCH`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switch<'a> {
    value: ControlFlowValue,
    cases: Vec<CaseEntry<'a>>,
}

impl<'a> Switch<'a> {
    /// Create a switch block with unified constructor.
    pub fn new<T>(value: ControlFlowValue, cases: T) -> Self
    where
        T: IntoIterator<Item = CaseEntry<'a>>,
    {
        Self {
            value,
            cases: cases.into_iter().collect(),
        }
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

    /// Get case by index.
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&CaseEntry<'a>> {
        self.cases.get(index)
    }

    /// Get mutable case by index.
    pub fn at_mut(&mut self, index: usize) -> Option<&mut CaseEntry<'a>> {
        self.cases.get_mut(index)
    }

    /// Convert the model into lex tokens representing the switch block.
    #[must_use]
    pub fn into_tokens(self) -> Vec<Token<'a>> {
        let mut out = Vec::new();
        match &self.value {
            ControlFlowValue::GenMax(max) => out.push(Token::Header {
                name: "SWITCH".into(),
                args: max.to_string().into(),
            }),
            ControlFlowValue::Set(val) => out.push(Token::Header {
                name: "SETSWITCH".into(),
                args: val.to_string().into(),
            }),
        }

        self.cases.into_iter().for_each(|case| {
            out.extend(
                std::iter::once(case.condition.map_or_else(
                    || Token::Header {
                        name: "DEF".into(),
                        args: "".into(),
                    },
                    |cond| Token::Header {
                        name: "CASE".into(),
                        args: cond.to_string().into(),
                    },
                ))
                .chain(case.units.into_iter().flat_map(TokenUnit::into_tokens))
                .chain(case.skip.then(|| Token::Header {
                    name: "SKIP".into(),
                    args: "".into(),
                })),
            );
        });

        out.push(Token::Header {
            name: "ENDSW".into(),
            args: "".into(),
        });

        out
    }
}

impl<'a> IntoIterator for Switch<'a> {
    type Item = CaseEntry<'a>;
    type IntoIter = std::vec::IntoIter<CaseEntry<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.cases.into_iter()
    }
}

impl<'b, 'a> IntoIterator for &'b Switch<'a> {
    type Item = &'b CaseEntry<'a>;
    type IntoIter = std::slice::Iter<'b, CaseEntry<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.cases.iter()
    }
}

impl<'a> Index<usize> for Switch<'a> {
    type Output = CaseEntry<'a>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.cases[index]
    }
}

impl<'a> IndexMut<usize> for Switch<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.cases[index]
    }
}

/// Builder for `Switch`, supporting chained `case`/`def` construction.
#[derive(Debug, Clone)]
pub struct SwitchBuilder<'a> {
    value: ControlFlowValue,
    cases: Vec<CaseEntry<'a>>,
}

impl<'a> SwitchBuilder<'a> {
    /// Create a builder with provided control-flow value.
    #[must_use]
    pub const fn new(value: ControlFlowValue) -> Self {
        Self {
            value,
            cases: Vec::new(),
        }
    }

    /// Add a `#CASE <cond>` branch with units. `skip` defaults to true.
    pub fn case<U>(mut self, cond: BigUint, units: U) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        self.cases.push(CaseEntry::new(cond, units));
        self
    }

    /// Add a `#CASE <cond>` branch with explicit `skip` control.
    pub fn case_with_skip<U>(mut self, cond: BigUint, units: U, skip: bool) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        let mut entry = CaseEntry::new(cond, units);
        entry.set_skip(skip);
        self.cases.push(entry);
        self
    }

    // (removed token-based aliases; all case methods now accept units)

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

    // (removed token-based aliases; all def methods now accept units)

    /// Push a prepared `CaseEntry` into builder.
    #[must_use]
    pub fn push_case(mut self, entry: CaseEntry<'a>) -> Self {
        self.cases.push(entry);
        self
    }

    /// Finalize builder into a `Switch` model.
    #[must_use]
    pub fn build(self) -> Switch<'a> {
        Switch {
            value: self.value,
            cases: self.cases,
        }
    }
}
