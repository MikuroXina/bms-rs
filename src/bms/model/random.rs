//! Random model: `If` and unified `Random` structure.
//!
//! This module defines a lightweight model to build `#RANDOM`/`#SETRANDOM` blocks
//! using regular BMS tokens. Branch entries accept tokens with any lifetime
//! (`Token<'a>`), so you can construct random blocks from borrowed strings
//! without requiring `'static` data.

use std::ops::{Index, IndexMut};

use num::BigUint;

use crate::bms::lex::token::Token;

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
    tokens: Vec<Token<'a>>, // exclude control-flow tokens
}

impl<'a> IfChainEntry<'a> {
    fn new<T>(condition: Option<BigUint>, tokens: T) -> Self
    where
        T: IntoIterator<Item = Token<'a>>,
    {
        // Filter out control-flow tokens defensively to keep model invariant.
        let filtered = tokens
            .into_iter()
            .filter(|t| match t {
                Token::Header { .. } => !t.is_control_flow_token(),
                _ => true,
            })
            .collect();
        Self {
            condition,
            tokens: filtered,
        }
    }

    /// Returns the condition if present (None for `else`).
    pub fn condition(&self) -> Option<&BigUint> {
        self.condition.as_ref()
    }

    /// Returns a view of the non-control tokens contained in this branch.
    pub fn tokens(&self) -> &[Token<'a>] {
        &self.tokens
    }

    /// Set a new condition for this entry.
    /// Returns the previous condition when this entry had a condition,
    /// or None if this is an `else` entry (no change is applied).
    pub fn set_condition(&mut self, new_condition: BigUint) -> Option<BigUint> {
        match self.condition.as_mut() {
            Some(cond) => Some(std::mem::replace(cond, new_condition)),
            None => None, // else-branch keeps None
        }
    }

    /// Replace tokens of this entry (control-flow tokens are filtered out).
    /// Returns the previous tokens.
    pub fn set_tokens<T>(&mut self, new_tokens: T) -> Vec<Token<'a>>
    where
        T: IntoIterator<Item = Token<'a>>,
    {
        let mut filtered: Vec<Token<'a>> = new_tokens
            .into_iter()
            .filter(|t| match t {
                Token::Header { .. } => !t.is_control_flow_token(),
                _ => true,
            })
            .collect();
        std::mem::swap(&mut filtered, &mut self.tokens);
        filtered
    }
}

/// If-chain used within a random block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct If<'a> {
    entries: Vec<IfChainEntry<'a>>,
}

impl<'a> If<'a> {
    /// Create a new if-chain with a single `if` entry.
    pub fn new<T>(cond: BigUint, tokens: T) -> Self
    where
        T: IntoIterator<Item = Token<'a>>,
    {
        Self {
            entries: vec![IfChainEntry::new(Some(cond), tokens)],
        }
    }

    /// Add an `else if` entry to the chain.
    pub fn or_else_if<T>(mut self, cond: BigUint, tokens: T) -> Self
    where
        T: IntoIterator<Item = Token<'a>>,
    {
        self.entries.push(IfChainEntry::new(Some(cond), tokens));
        self
    }

    /// Add an `else` entry to the chain.
    pub fn or_else<T>(mut self, tokens: T) -> Self
    where
        T: IntoIterator<Item = Token<'a>>,
    {
        self.entries.push(IfChainEntry::new(None, tokens));
        self
    }

    /// Get an entry by index.
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
    pub fn len(&self) -> usize {
        self.branches.len()
    }

    /// Returns true if there are no branches in this random block.
    pub fn is_empty(&self) -> bool {
        self.branches.is_empty()
    }

    /// Get branch by index.
    pub fn at(&self, index: usize) -> Option<&If<'a>> {
        self.branches.get(index)
    }

    /// Get mutable branch by index.
    pub fn at_mut(&mut self, index: usize) -> Option<&mut If<'a>> {
        self.branches.get_mut(index)
    }

    /// Convert the model into lex tokens representing the random block.
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

        for branch in self.branches {
            let mut is_first = true;
            for entry in branch.entries {
                match (is_first, entry.condition) {
                    (true, Some(cond)) => {
                        out.push(Token::Header {
                            name: "IF".into(),
                            args: cond.to_string().into(),
                        });
                    }
                    (false, Some(cond)) => {
                        out.push(Token::Header {
                            name: "ELSEIF".into(),
                            args: cond.to_string().into(),
                        });
                    }
                    (_, None) => {
                        out.push(Token::Header {
                            name: "ELSE".into(),
                            args: "".into(),
                        });
                    }
                }

                out.extend(entry.tokens);
                is_first = false;
            }

            out.push(Token::Header {
                name: "ENDIF".into(),
                args: "".into(),
            });
        }

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
