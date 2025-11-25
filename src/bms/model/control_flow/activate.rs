//! Activation module for control-flow models.
//!
//! This module provides a trait to evaluate `Random`/`Switch` control-flow
//! structures using an `Rng` and produce activated tokens with source ranges.

use std::ops::RangeInclusive;

use num::BigUint;

use crate::bms::lex::token::{Token, TokenWithRange};

use crate::bms::rng::Rng;

use crate::bms::command::mixin::SourceRangeMixin;

use super::{ControlFlowValue, IfChainEntry, Random, Switch, TokenUnit};

/// Activates control-flow structures into a vector of `TokenWithRange` using an RNG.
pub trait Activate<'a> {
    /// Evaluate control-flow and return the activated tokens.
    fn activate<R: Rng>(self, rng: &mut R) -> Vec<TokenWithRange<'a>>;
}

impl<'a> Activate<'a> for TokenUnit<'a> {
    fn activate<R: Rng>(self, rng: &mut R) -> Vec<TokenWithRange<'a>> {
        match self {
            TokenUnit::Random(r) => Activate::activate(r, rng),
            TokenUnit::Switch(s) => Activate::activate(s, rng),
            TokenUnit::Tokens(v) => v
                .into_iter()
                .map(Token::from)
                .map(|t| SourceRangeMixin::new(t, 0..0))
                .collect(),
        }
    }
}

impl<'a> Activate<'a> for Random<'a> {
    fn activate<R: Rng>(self, rng: &mut R) -> Vec<TokenWithRange<'a>> {
        let generated = match self.value {
            ControlFlowValue::GenMax(max) => {
                let range: RangeInclusive<BigUint> = BigUint::from(1u64)..=max;
                rng.generate(range)
            }
            ControlFlowValue::Set(val) => val,
        };

        let mut out: Vec<TokenWithRange<'a>> = Vec::new();
        for branch in self.branches.into_iter() {
            // head if
            if branch.condition == generated {
                out.extend(
                    branch
                        .head_units
                        .into_iter()
                        .flat_map(|u| Activate::activate(u, rng)),
                );
                continue;
            }

            // chain else-if / else
            let mut node = branch.chain;
            loop {
                match node {
                    IfChainEntry::ElseIf { cond, units, next } => {
                        if cond == generated {
                            out.extend(units.into_iter().flat_map(|u| Activate::activate(u, rng)));
                            break;
                        }
                        node = *next;
                    }
                    IfChainEntry::Else { units } => {
                        out.extend(units.into_iter().flat_map(|u| Activate::activate(u, rng)));
                        break;
                    }
                    IfChainEntry::EndIf => break,
                }
            }
        }
        out
    }
}

impl<'a> Activate<'a> for Switch<'a> {
    fn activate<R: Rng>(self, rng: &mut R) -> Vec<TokenWithRange<'a>> {
        let generated = match self.value {
            ControlFlowValue::GenMax(max) => {
                let range: RangeInclusive<BigUint> = BigUint::from(1u64)..=max;
                rng.generate(range)
            }
            ControlFlowValue::Set(val) => val,
        };

        // Prefer matched case; otherwise, use the last default.
        let mut out: Vec<TokenWithRange<'a>> = Vec::new();
        let mut default_units: Option<Vec<TokenUnit<'a>>> = None;
        for case in self.cases.into_iter() {
            match case.condition {
                Some(cond) if cond == generated => {
                    out.extend(
                        case.units
                            .into_iter()
                            .flat_map(|u| Activate::activate(u, rng)),
                    );
                    return out;
                }
                None => {
                    default_units = Some(case.units);
                }
                _ => {}
            }
        }

        if let Some(units) = default_units {
            out.extend(units.into_iter().flat_map(|u| Activate::activate(u, rng)));
        }
        out
    }
}
