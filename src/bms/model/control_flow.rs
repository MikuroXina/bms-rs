//! Random model: `RandomizedObjects` and `RandomizedBranch`.
//!
//! This module defines the model for `#RANDOM` and `#SWITCH` control flow structures.
//! Unlike the previous implementation which stored tokens, this model stores fully parsed `Bms` objects
//! for each branch, allowing for recursive evaluation.

use num::BigUint;

use crate::bms::model::Bms;
use crate::bms::prelude::*;
use crate::bms::rng::Rng;

/// Indicates whether the random block generates a value (`#RANDOM`) or uses a set value (`#SETRANDOM`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ControlFlowValue {
    /// Use a fixed value (emits `#SETRANDOM <value>`).
    Set(BigUint),
    /// Generate a random value with the given maximum (emits `#RANDOM <max>`).
    GenMax(BigUint),
}

/// A branch in a randomized block.
///
/// `condition = Some(n)` means a conditional branch (`#IF n` / `#CASE n` / `#ELSEIF n`).
/// `condition = None` means an unconditional fallback (`#ELSE` / `#DEF`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RandomizedBranch {
    /// The condition value for this branch.
    /// `Some(n)` = conditional branch (`#IF n` / `#CASE n` / `#ELSEIF n`).
    /// `None` = unconditional fallback (`#ELSE` / `#DEF`).
    pub condition: Option<BigUint>,
    /// The content of this branch, parsed as a BMS object.
    pub sub: Box<Bms>,
}

impl RandomizedBranch {
    /// Create a new randomized branch.
    #[must_use]
    pub fn new(condition: Option<BigUint>, sub: Bms) -> Self {
        Self {
            condition,
            sub: Box::new(sub),
        }
    }

    /// Get the condition value, if this is a conditional branch.
    #[must_use]
    pub const fn condition(&self) -> Option<&BigUint> {
        self.condition.as_ref()
    }

    /// Get the sub-BMS object.
    #[must_use]
    pub fn sub(&self) -> &Bms {
        &self.sub
    }

    /// Get a mutable reference to the sub-BMS object.
    ///
    /// This allows in-place modification of the branch content.
    pub fn sub_mut(&mut self) -> &mut Bms {
        &mut self.sub
    }
}

/// A collection of randomized branches, representing a `#RANDOM` or `#SWITCH` block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Randomized objects container for storing data associated with `#RANDOM`, `#SWITCH`, `#ENDRANDOM`, and `#ENDSWITCH`.
pub struct RandomizedObjects {
    /// The line number of the `#RANDOM` or `#SWITCH` token.
    pub line_number: usize,
    /// The value or max value for this random block.
    pub generating: Option<ControlFlowValue>,
    /// The branches for this random block.
    /// Conditional branches (with `Some(n)`) come first; fallback (`None`) last.
    pub branches: Vec<RandomizedBranch>,
}

impl RandomizedObjects {
    /// Create a new empty `RandomizedObjects`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another `RandomizedObjects` into this one.
    ///
    /// The `generating` field of `other` overwrites `self` if present.
    /// Branches from `other` are merged into `self`. If a branch with the same
    /// condition exists in both, their content `Bms` objects are unioned.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let generating = other.generating.clone().or_else(|| self.generating.clone());
        let mut branches = self.branches.clone();
        for branch in &other.branches {
            if let Some(existing) = branches
                .iter_mut()
                .find(|b| b.condition == branch.condition)
            {
                let merged_bms = existing.sub.union(&branch.sub);
                *existing.sub = merged_bms;
            } else {
                branches.push(branch.clone());
            }
        }
        Self {
            line_number: self.line_number,
            generating,
            branches,
        }
    }

    /// Merge another `RandomizedObjects` into this one in-place.
    pub fn union_inplace(&mut self, other: &Self) {
        if other.generating.is_some() {
            self.generating.clone_from(&other.generating);
        }
        for branch in &other.branches {
            if let Some(existing) = self
                .branches
                .iter_mut()
                .find(|b| b.condition == branch.condition)
            {
                existing.sub.union_inplace(&branch.sub);
            } else {
                self.branches.push(branch.clone());
            }
        }
    }

    /// Returns the current generator configuration for this randomized block.
    ///
    /// `None` indicates the block does not have a generator assigned yet.
    #[must_use]
    pub const fn generating(&self) -> Option<&ControlFlowValue> {
        self.generating.as_ref()
    }

    /// Sets the generator configuration for this randomized block.
    ///
    /// Overwrites any previously set generator.
    pub fn set_generating(&mut self, generating: ControlFlowValue) {
        self.generating = Some(generating);
    }

    /// Returns an immutable reference to the branch with the given condition value.
    ///
    /// The `condition` typically corresponds to `#IF n` or `#CASE n`.
    #[must_use]
    pub fn branch(&self, condition: &BigUint) -> Option<&RandomizedBranch> {
        self.branches
            .iter()
            .find(|b| b.condition.as_ref() == Some(condition))
    }

    /// Returns a mutable reference to the branch with the given condition value.
    ///
    /// The `condition` typically corresponds to `#IF n` or `#CASE n`.
    pub fn branch_mut(&mut self, condition: &BigUint) -> Option<&mut RandomizedBranch> {
        self.branches
            .iter_mut()
            .find(|b| b.condition.as_ref() == Some(condition))
    }

    /// Returns a mutable reference to the branch entry for `condition`.
    ///
    /// If the branch does not exist, a new one with an empty `Bms` is inserted and returned.
    ///
    /// # Panics
    ///
    /// Never panics under normal operation. The internal push-then-pop sequence
    /// is guaranteed to succeed.
    #[expect(clippy::indexing_slicing)]
    pub fn branch_entry(&mut self, condition: BigUint) -> &mut RandomizedBranch {
        if let Some(pos) = self
            .branches
            .iter()
            .position(|b| b.condition.as_ref() == Some(&condition))
        {
            &mut self.branches[pos]
        } else {
            self.branches
                .push(RandomizedBranch::new(Some(condition), Bms::default()));
            self.branches.last_mut().expect("just pushed an element")
        }
    }

    /// Returns an iterator over all branches in insertion order.
    pub fn branches(&self) -> impl Iterator<Item = &RandomizedBranch> {
        self.branches.iter()
    }

    /// Returns a mutable iterator over all branches in insertion order.
    pub fn branches_mut(&mut self) -> impl Iterator<Item = &mut RandomizedBranch> {
        self.branches.iter_mut()
    }

    /// Removes branches whose `sub` content is an empty `Bms`.
    ///
    /// Useful for cleaning up placeholder branches created during parsing or editing.
    pub fn prune_branches(&mut self) {
        self.branches.retain(|b| *b.sub != Bms::default());
    }

    /// Evaluate the random structure and return the selected branch's content.
    ///
    /// This resolves the `#RANDOM` or `#SWITCH` logic using the provided `rng`.
    /// If a branch is selected, its `Bms` content is returned.
    /// If the selected branch's content also contains randomized objects, they are NOT automatically evaluated by this method.
    /// You may need to call `evaluate` recursively on the result if you want full resolution.
    pub fn evaluate(&self, mut rng: impl Rng) -> Bms {
        let val = match &self.generating {
            Some(ControlFlowValue::Set(v)) => v.clone(),
            Some(ControlFlowValue::GenMax(max)) => rng.generate(BigUint::from(1u64)..=max.clone()),
            None => return Bms::default(),
        };

        for branch in &self.branches {
            if branch.condition.as_ref() == Some(&val) {
                return (*branch.sub).clone();
            }
        }

        for branch in &self.branches {
            if branch.condition.is_none() {
                return (*branch.sub).clone();
            }
        }

        Bms::default()
    }

    /// Exports this randomized block as `#RANDOM/#SETRANDOM + #IF/#ELSEIF/#ELSE/#ENDIF` tokens.
    ///
    /// Conditional branches are sorted by condition value.
    /// The fallback branch (`condition: None`) is emitted as `#ELSE` after all conditional branches.
    ///
    /// # Panics
    ///
    /// Panics if a conditional branch's condition is unexpectedly `None`. This
    /// cannot happen under normal operation because the filter guarantees `Some`.
    #[must_use]
    pub fn export_as_random<'a, T: KeyLayoutMapper>(&'a self) -> Vec<Token<'a>> {
        let mut tokens: Vec<Token<'a>> = Vec::new();

        let Some(gval) = &self.generating else {
            return tokens;
        };

        match gval {
            ControlFlowValue::Set(v) => tokens.push(Token::Header {
                name: "SETRANDOM".into(),
                args: v.to_string().into(),
            }),
            ControlFlowValue::GenMax(m) => tokens.push(Token::Header {
                name: "RANDOM".into(),
                args: m.to_string().into(),
            }),
        }

        let mut conditional: Vec<&RandomizedBranch> = self
            .branches
            .iter()
            .filter(|b| b.condition.is_some())
            .collect();
        conditional.sort_by(|a, b| a.condition.cmp(&b.condition));
        let fallback = self.branches.iter().find(|b| b.condition.is_none());

        if let Some(branch) = conditional.first() {
            let cond = branch
                .condition
                .as_ref()
                .expect("conditional branches have Some condition");
            tokens.push(Token::Header {
                name: "IF".into(),
                args: cond.to_string().into(),
            });
            tokens.extend(branch.sub.as_ref().unparse::<T>());
        }
        for branch in conditional.iter().skip(1) {
            let cond = branch
                .condition
                .as_ref()
                .expect("conditional branches have Some condition");
            tokens.push(Token::Header {
                name: "ELSEIF".into(),
                args: cond.to_string().into(),
            });
            tokens.extend(branch.sub.as_ref().unparse::<T>());
        }
        if let Some(branch) = fallback {
            tokens.push(Token::Header {
                name: "ELSE".into(),
                args: "".into(),
            });
            tokens.extend(branch.sub.as_ref().unparse::<T>());
        }

        tokens.push(Token::Header {
            name: "ENDIF".into(),
            args: "".into(),
        });
        tokens.push(Token::Header {
            name: "ENDRANDOM".into(),
            args: "".into(),
        });

        tokens
    }

    /// Exports this randomized block as `#SWITCH/#SETSWITCH + #CASE/#SKIP/#DEF/#ENDSW` tokens.
    ///
    /// Conditional branches are sorted by condition value.
    /// The fallback branch (`condition: None`) is emitted as `#DEF` after all conditional branches.
    ///
    /// # Panics
    ///
    /// Panics if a conditional branch's condition is unexpectedly `None`. This
    /// cannot happen under normal operation because the filter guarantees `Some`.
    #[must_use]
    pub fn export_as_switch<'a, T: KeyLayoutMapper>(&'a self) -> Vec<Token<'a>> {
        let mut tokens: Vec<Token<'a>> = Vec::new();

        let Some(gval) = &self.generating else {
            return tokens;
        };

        match gval {
            ControlFlowValue::Set(v) => tokens.push(Token::Header {
                name: "SETSWITCH".into(),
                args: v.to_string().into(),
            }),
            ControlFlowValue::GenMax(m) => tokens.push(Token::Header {
                name: "SWITCH".into(),
                args: m.to_string().into(),
            }),
        }

        let mut conditional: Vec<&RandomizedBranch> = self
            .branches
            .iter()
            .filter(|b| b.condition.is_some())
            .collect();
        conditional.sort_by(|a, b| a.condition.cmp(&b.condition));
        let fallback = self.branches.iter().find(|b| b.condition.is_none());

        for branch in &conditional {
            let cond = branch
                .condition
                .as_ref()
                .expect("conditional branches have Some condition");
            tokens.push(Token::Header {
                name: "CASE".into(),
                args: cond.to_string().into(),
            });
            tokens.extend(branch.sub.as_ref().unparse::<T>());
            tokens.push(Token::Header {
                name: "SKIP".into(),
                args: "".into(),
            });
        }

        if let Some(branch) = fallback {
            tokens.push(Token::Header {
                name: "DEF".into(),
                args: "".into(),
            });
            tokens.extend(branch.sub.as_ref().unparse::<T>());
            tokens.push(Token::Header {
                name: "SKIP".into(),
                args: "".into(),
            });
        }

        tokens.push(Token::Header {
            name: "ENDSW".into(),
            args: "".into(),
        });

        tokens
    }
}
