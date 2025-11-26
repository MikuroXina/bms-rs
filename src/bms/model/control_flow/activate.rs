//! Activation module for control-flow models.
//!
//! This module provides a trait to evaluate `Random`/`Switch` control-flow
//! structures using an `Rng` and produce activated tokens with source ranges.

use std::ops::RangeInclusive;

use num::BigUint;

use crate::bms::lex::token::{Token, TokenWithRange};

use crate::bms::rng::Rng;

use crate::bms::command::mixin::SourceRangeMixin;
use crate::bms::parse::{ControlFlowError, ControlFlowErrorWithRange};

use super::{ControlFlowValue, IfChainEntry, Random, Switch, TokenUnit};

/// Activates control-flow structures using an RNG, returning activated tokens.
pub trait Activate<'a> {
    /// Evaluate control-flow and return activated `TokenWithRange`s or an error.
    fn activate<R: Rng>(
        self,
        rng: &mut R,
    ) -> Result<Vec<TokenWithRange<'a>>, ControlFlowErrorWithRange>;
}

impl<'a> Activate<'a> for TokenUnit<'a> {
    fn activate<R: Rng>(
        self,
        rng: &mut R,
    ) -> Result<Vec<TokenWithRange<'a>>, ControlFlowErrorWithRange> {
        match self {
            TokenUnit::Random(r) => Activate::activate(r, rng),
            TokenUnit::Switch(s) => Activate::activate(s, rng),
            TokenUnit::Tokens(v) => Ok(v
                .into_iter()
                .map(Token::from)
                .map(|t| SourceRangeMixin::new(t, 0..0))
                .collect()),
        }
    }
}

impl<'a> Activate<'a> for Random<'a> {
    fn activate<R: Rng>(
        self,
        rng: &mut R,
    ) -> Result<Vec<TokenWithRange<'a>>, ControlFlowErrorWithRange> {
        let generated = match self.value {
            ControlFlowValue::GenMax(max) => {
                let range: RangeInclusive<BigUint> = BigUint::from(1u64)..=max;
                let g = rng.generate(range.clone());
                range.contains(&g).then_some(g.clone()).ok_or_else(|| {
                    SourceRangeMixin::new(
                        ControlFlowError::RandomGeneratedValueOutOfRange {
                            expected: range,
                            actual: g,
                        },
                        0..0,
                    )
                })?
            }
            ControlFlowValue::Set(val) => val,
        };

        let mut out: Vec<TokenWithRange<'a>> = Vec::new();
        for branch in self.branches.into_iter() {
            if branch.condition == generated {
                for u in branch.head_units.into_iter() {
                    let tokens = Activate::activate(u, rng)?;
                    out.extend(tokens);
                }
                continue;
            }

            let mut node = branch.chain;
            loop {
                match node {
                    IfChainEntry::ElseIf { cond, units, next } => {
                        if cond == generated {
                            for u in units.into_iter() {
                                let tokens = Activate::activate(u, rng)?;
                                out.extend(tokens);
                            }
                            break;
                        }
                        node = *next;
                    }
                    IfChainEntry::Else { units } => {
                        for u in units.into_iter() {
                            let tokens = Activate::activate(u, rng)?;
                            out.extend(tokens);
                        }
                        break;
                    }
                    IfChainEntry::EndIf => break,
                }
            }
        }
        Ok(out)
    }
}

impl<'a> Activate<'a> for Switch<'a> {
    fn activate<R: Rng>(
        self,
        rng: &mut R,
    ) -> Result<Vec<TokenWithRange<'a>>, ControlFlowErrorWithRange> {
        let generated = match self.value {
            ControlFlowValue::GenMax(max) => {
                let range: RangeInclusive<BigUint> = BigUint::from(1u64)..=max;
                let g = rng.generate(range.clone());
                range.contains(&g).then_some(g.clone()).ok_or_else(|| {
                    SourceRangeMixin::new(
                        ControlFlowError::SwitchGeneratedValueOutOfRange {
                            expected: range,
                            actual: g,
                        },
                        0..0,
                    )
                })?
            }
            ControlFlowValue::Set(val) => val,
        };

        let cases = self.cases;
        let mut matched_index: Option<usize> = None;
        let mut last_default_index: Option<usize> = None;
        let mut seen_case = false;
        let mut first_default_before_any_case: Option<usize> = None;

        for (idx, case) in cases.iter().enumerate() {
            match case.condition {
                Some(ref cond) => {
                    seen_case = true;
                    if *cond == generated {
                        matched_index = Some(idx);
                        break;
                    }
                }
                None => {
                    last_default_index = Some(idx);
                    if !seen_case && first_default_before_any_case.is_none() {
                        first_default_before_any_case = Some(idx);
                    }
                }
            }
        }

        let mut target_index: Option<usize> = None;
        let mut is_fallthrough = false;

        if let Some(i) = first_default_before_any_case {
            target_index = Some(i);
        } else if let Some(i) = matched_index {
            target_index = Some(i);
            is_fallthrough = true;
        } else if let Some(i) = last_default_index {
            target_index = Some(i);
        }

        let mut out: Vec<TokenWithRange<'a>> = Vec::new();

        if let Some(start_idx) = target_index {
            let iter = cases.into_iter().skip(start_idx);

            if is_fallthrough {
                for case in iter {
                    for u in case.units {
                        let tokens = Activate::activate(u, rng)?;
                        out.extend(tokens);
                    }
                    if case.skip {
                        break;
                    }
                }
            } else if let Some(case) = iter.into_iter().next() {
                for u in case.units {
                    let tokens = Activate::activate(u, rng)?;
                    out.extend(tokens);
                }
            }
        }

        Ok(out)
    }
}
