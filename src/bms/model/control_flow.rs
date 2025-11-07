//! Random model: `IfBlock` and unified `Random` structure.
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

/// One branch node in an if-chain inside a random block.
#[derive(Debug, Clone, PartialEq, Eq)]
enum IfChainEntry<'a> {
    /// An `#ELSEIF <cond>` branch with its units and a pointer to the next entry.
    ElseIf {
        /// Condition value for `#ELSEIF <cond>`.
        cond: BigUint,
        /// Content units for this branch.
        units: Vec<TokenUnit<'a>>,
        /// Pointer to the next chain entry.
        next: Box<IfChainEntry<'a>>,
    },
    /// An `#ELSE` branch with its units and a pointer to the next entry.
    Else {
        /// Content units for this branch.
        units: Vec<TokenUnit<'a>>,
        /// Pointer to the next chain entry.
        next: Box<IfChainEntry<'a>>,
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
            IfChainEntry::ElseIf { next, .. } | IfChainEntry::Else { next, .. } => {
                let mut cur = next.as_mut();
                loop {
                    match cur {
                        IfChainEntry::EndIf => {
                            *cur = entry;
                            break;
                        }
                        IfChainEntry::ElseIf { next, .. } | IfChainEntry::Else { next, .. } => {
                            cur = next.as_mut();
                        }
                    }
                }
            }
        }
    }

    fn units_at(&self, chain_index: usize) -> Option<&[TokenUnit<'a>]> {
        // chain_index refers to entries after the head: 1 => first else-if/else
        let mut idx = chain_index;
        let mut cur = self;
        while idx > 0 {
            match cur {
                IfChainEntry::ElseIf { next, .. } | IfChainEntry::Else { next, .. } => {
                    idx -= 1;
                    cur = next.as_ref();
                }
                IfChainEntry::EndIf => return None,
            }
        }
        match cur {
            IfChainEntry::ElseIf { units, .. } | IfChainEntry::Else { units, .. } => Some(units),
            IfChainEntry::EndIf => None,
        }
    }

    fn set_units_at<U>(&mut self, chain_index: usize, new_units: U) -> Option<Vec<TokenUnit<'a>>>
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        let mut idx = chain_index;
        let mut cur: *mut IfChainEntry<'a> = self as *mut _;
        unsafe {
            loop {
                match &mut *cur {
                    IfChainEntry::ElseIf { units, next, .. }
                    | IfChainEntry::Else { units, next, .. } => {
                        if idx == 0 {
                            let mut incoming: Vec<TokenUnit<'a>> = new_units.into_iter().collect();
                            std::mem::swap(&mut incoming, units);
                            return Some(incoming);
                        } else {
                            idx -= 1;
                            cur = next.as_mut() as *mut _;
                        }
                    }
                    IfChainEntry::EndIf => return None,
                }
            }
        }
    }
}

/// If-chain used within a random block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfBlock<'a> {
    condition: BigUint,
    head_units: Vec<TokenUnit<'a>>, // units for the initial `#IF` branch
    chain: IfChainEntry<'a>,        // subsequent `ELSEIF`/`ELSE` nodes
}

impl<'a> IfBlock<'a> {
    /// Create a new if-chain with units in the first `#IF` entry.
    pub fn new_if<U>(cond: BigUint, units: U) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        Self {
            condition: cond,
            head_units: units.into_iter().collect(),
            chain: IfChainEntry::EndIf,
        }
    }

    /// Add an `#ELSEIF <cond>` entry with units.
    pub fn or_else_if<U>(mut self, cond: BigUint, units: U) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        let entry = IfChainEntry::ElseIf {
            cond,
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
            next: Box::new(IfChainEntry::EndIf),
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
            let mut incoming: Vec<TokenUnit<'a>> = new_units.into_iter().collect();
            std::mem::swap(&mut incoming, &mut self.head_units);
            Some(incoming)
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
    pub const fn set_condition(&mut self, new_condition: BigUint) -> BigUint {
        std::mem::replace(&mut self.condition, new_condition)
    }

    /// Returns the number of branches in this if-chain (including head `#IF`).
    #[must_use]
    pub fn len(&self) -> usize {
        let mut count = 1; // head if
        let mut cur = &self.chain;
        while let IfChainEntry::ElseIf { next, .. } | IfChainEntry::Else { next, .. } = cur {
            count += 1;
            cur = next.as_ref();
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
    value: ControlFlowValue,
    branches: Vec<IfBlock<'a>>,
}

impl<'a> Random<'a> {
    /// Create an empty random block (`#RANDOM` or `#SETRANDOM`).
    #[must_use]
    pub const fn new(value: ControlFlowValue) -> Self {
        Self {
            value,
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
            // Emit head IF
            out.push(Token::Header {
                name: "IF".into(),
                args: branch.condition.to_string().into(),
            });
            out.extend(
                branch
                    .head_units
                    .into_iter()
                    .flat_map(TokenUnit::into_tokens),
            );

            // Emit chained ELSEIF/ELSE entries
            let mut node = branch.chain;
            loop {
                match node {
                    IfChainEntry::ElseIf { cond, units, next } => {
                        out.push(Token::Header {
                            name: "ELSEIF".into(),
                            args: cond.to_string().into(),
                        });
                        out.extend(units.into_iter().flat_map(TokenUnit::into_tokens));
                        node = *next;
                    }
                    IfChainEntry::Else { units, next } => {
                        out.push(Token::Header {
                            name: "ELSE".into(),
                            args: "".into(),
                        });
                        out.extend(units.into_iter().flat_map(TokenUnit::into_tokens));
                        node = *next;
                    }
                    IfChainEntry::EndIf => break,
                }
            }

            // Close the IF-chain
            out.push(Token::Header {
                name: "ENDIF".into(),
                args: "".into(),
            });
        });

        out.push(Token::Header {
            name: "ENDRANDOM".into(),
            args: "".into(),
        });

        out
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
}

/// A switch block (`#SWITCH` or `#SETSWITCH`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switch<'a> {
    value: ControlFlowValue,
    cases: Vec<CaseEntry<'a>>,
}

impl<'a> Switch<'a> {
    /// Start building a `Switch` with a control-flow value.
    /// This returns an empty `Switch` to be populated via builder-style methods.
    #[must_use]
    pub const fn new(value: ControlFlowValue) -> Self {
        Self {
            value,
            cases: Vec::new(),
        }
    }

    /// Add a `#CASE <cond>` branch and emit `#SKIP` after tokens.
    pub fn case_with_skip<U>(mut self, cond: BigUint, units: U) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        let mut entry = CaseEntry::new(cond, units);
        entry.set_skip(true);
        self.cases.push(entry);
        self
    }

    /// Add a `#CASE <cond>` branch and do not emit `#SKIP` after tokens.
    pub fn case_no_skip<U>(mut self, cond: BigUint, units: U) -> Self
    where
        U: IntoIterator<Item = TokenUnit<'a>>,
    {
        let mut entry = CaseEntry::new(cond, units);
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
