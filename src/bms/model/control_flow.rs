//! Random model: `RandomizedObjects` and `RandomizedBranch`.
//!
//! This module defines the model for `#RANDOM` and `#SWITCH` control flow structures.
//! Unlike the previous implementation which stored tokens, this model stores fully parsed `Bms` objects
//! for each branch, allowing for recursive evaluation.

use std::collections::BTreeMap;

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
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RandomizedBranch {
    /// The condition value for this branch (e.g. `1` for `#IF 1` or `#CASE 1`).
    pub condition: BigUint,
    /// The content of this branch, parsed as a BMS object.
    pub sub: Box<Bms>,
}

impl RandomizedBranch {
    /// Create a new randomized branch.
    #[must_use]
    pub fn new(condition: BigUint, sub: Bms) -> Self {
        Self {
            condition,
            sub: Box::new(sub),
        }
    }

    /// Get the condition value.
    #[must_use]
    pub const fn condition(&self) -> &BigUint {
        &self.condition
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
pub struct RandomizedObjects {
    /// The control flow value generator.
    pub generating: Option<ControlFlowValue>,
    /// The branches, mapped by their condition value.
    pub branches: BTreeMap<BigUint, RandomizedBranch>,
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
    /// Branches from `other` are merged into `self`. If a branch exists in both,
    /// their content `Bms` objects are unioned.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let generating = other.generating.clone().or_else(|| self.generating.clone());
        let mut branches = self.branches.clone();
        for (cond, branch) in &other.branches {
            branches
                .entry(cond.clone())
                .and_modify(|e| {
                    let merged_bms = e.sub.union(*branch.sub.clone());
                    *e.sub = merged_bms;
                })
                .or_insert_with(|| branch.clone());
        }
        Self {
            generating,
            branches,
        }
    }

    /// Merge another `RandomizedObjects` into this one in-place.
    pub fn union_inplace(&mut self, other: &Self) {
        if other.generating.is_some() {
            self.generating = other.generating.clone();
        }
        for (cond, branch) in &other.branches {
            self.branches
                .entry(cond.clone())
                .and_modify(|e| {
                    e.sub.union_inplace(&branch.sub);
                })
                .or_insert_with(|| branch.clone());
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
    pub fn branch(&self, condition: BigUint) -> Option<&RandomizedBranch> {
        self.branches.get(&condition)
    }

    /// Returns a mutable reference to the branch with the given condition value.
    ///
    /// The `condition` typically corresponds to `#IF n` or `#CASE n`.
    pub fn branch_mut(&mut self, condition: BigUint) -> Option<&mut RandomizedBranch> {
        self.branches.get_mut(&condition)
    }

    /// Returns a mutable reference to the branch entry for `condition`.
    ///
    /// If the branch does not exist, a new one with an empty `Bms` is inserted and returned.
    pub fn branch_entry(&mut self, condition: BigUint) -> &mut RandomizedBranch {
        use std::collections::btree_map::Entry;
        match self.branches.entry(condition.clone()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(RandomizedBranch::new(condition, Bms::default())),
        }
    }

    /// Returns an iterator over all branches in ascending condition order.
    pub fn branches(&self) -> impl Iterator<Item = &RandomizedBranch> {
        self.branches.values()
    }

    /// Returns a mutable iterator over all branches in ascending condition order.
    pub fn branches_mut(&mut self) -> impl Iterator<Item = &mut RandomizedBranch> {
        self.branches.values_mut()
    }

    /// Removes branches whose `sub` content is an empty `Bms`.
    ///
    /// Useful for cleaning up placeholder branches created during parsing or editing.
    pub fn prune_branches(&mut self) {
        self.branches.retain(|_, b| *b.sub != Bms::default());
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

        if let Some(branch) = self.branches.get(&val) {
            *branch.sub.clone()
        } else {
            Bms::default()
        }
    }

    /// Exports this randomized block as `#RANDOM/#SETRANDOM + #IF/#ELSEIF/#ENDIF` tokens.
    ///
    /// Branch contents are exported via `Bms::unparse::<T>()` for each branch in condition order.
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

        let mut iter = self.branches.iter();
        if let Some((cond, branch)) = iter.next() {
            tokens.push(Token::Header {
                name: "IF".into(),
                args: cond.to_string().into(),
            });
            tokens.extend(branch.sub.unparse::<T>());
        }
        for (cond, branch) in iter {
            tokens.push(Token::Header {
                name: "ELSEIF".into(),
                args: cond.to_string().into(),
            });
            tokens.extend(branch.sub.unparse::<T>());
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

    /// Exports this randomized block as `#SWITCH/#SETSWITCH + #CASE/#SKIP/#ENDSW` tokens.
    ///
    /// Branch contents are exported via `Bms::unparse::<T>()` for each `#CASE` in condition order.
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

        for (cond, branch) in self.branches.iter() {
            tokens.push(Token::Header {
                name: "CASE".into(),
                args: cond.to_string().into(),
            });
            tokens.extend(branch.sub.unparse::<T>());
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
