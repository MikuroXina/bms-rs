//! Activation module for control-flow models.
//!
//! This module provides a trait to evaluate `Random`/`Switch` control-flow
//! structures using an `Rng` and produce activated borrowed token views
//! (`MaybeWithRange<&Token>`). Downstream processors that require
//! `TokenWithRange` can rewrap these views on demand.

use std::ops::RangeInclusive;

use num::BigUint;

use crate::bms::command::mixin::MaybeWithRange;
use crate::bms::lex::token::Token;

use crate::bms::rng::Rng;

use crate::bms::command::mixin::SourceRangeMixin;
use crate::bms::parse::{ControlFlowError, ControlFlowErrorWithRange};

use super::{ControlFlowValue, IfChainEntry, Random, Switch, TokenUnit};

/// Activates control-flow structures using an RNG, returning activated borrowed views.
pub trait Activate<'a> {
    /// Evaluates control-flow (`Random`/`Switch`) with an RNG and
    /// produces activated borrowed views with optional source ranges.
    fn activate<R: Rng>(
        self,
        rng: &mut R,
    ) -> Result<Vec<MaybeWithRange<&'a Token<'a>>>, ControlFlowErrorWithRange>;
}

impl<'a> Activate<'a> for TokenUnit<'a> {
    fn activate<R: Rng>(
        self,
        rng: &mut R,
    ) -> Result<Vec<MaybeWithRange<&'a Token<'a>>>, ControlFlowErrorWithRange> {
        match self {
            TokenUnit::Random(r) => Activate::activate(r, rng),
            TokenUnit::Switch(s) => Activate::activate(s, rng),
            TokenUnit::Tokens(v) => Ok(v
                .into_iter()
                .filter_map(|nc| match nc {
                    super::NonControlToken::Borrowed(view) => Some(view),
                    super::NonControlToken::Owned(_) => None,
                })
                .collect()),
        }
    }
}

impl<'a> Activate<'a> for Random<'a> {
    fn activate<R: Rng>(
        self,
        rng: &mut R,
    ) -> Result<Vec<MaybeWithRange<&'a Token<'a>>>, ControlFlowErrorWithRange> {
        // Generate or use the provided value, validating RNG output range.
        let value_wr = self.value.into_wrapped_or(0..0);
        let value_range = value_wr.range().clone();
        let generated = match value_wr.into_content() {
            ControlFlowValue::GenMax(max) => {
                let range: RangeInclusive<BigUint> = BigUint::from(1u64)..=max;
                let g = rng.generate(range.clone());
                range.contains(&g).then_some(g.clone()).ok_or_else(|| {
                    SourceRangeMixin::new(
                        ControlFlowError::RandomGeneratedValueOutOfRange {
                            expected: range,
                            actual: g,
                        },
                        value_range.clone(),
                    )
                })?
            }
            ControlFlowValue::Set(val) => val,
        };

        // Helper to activate nested units and append results, keeping code flat.
        let mut out: Vec<MaybeWithRange<&'a Token<'a>>> = Vec::new();
        let mut emit_units = |units: Vec<TokenUnit<'a>>| -> Result<(), ControlFlowErrorWithRange> {
            for u in units {
                let tokens = Activate::activate(u, rng)?;
                out.extend(tokens);
            }
            Ok(())
        };

        // Evaluate IF/ELSEIF/ELSE chains, emitting matching blocks.
        for branch in self.branches {
            if *branch.condition.content() == generated {
                emit_units(branch.head_units)?;
                continue;
            }

            let mut node = branch.chain;
            loop {
                match node {
                    IfChainEntry::ElseIf { cond, units, next } => {
                        if *cond.content() == generated {
                            emit_units(units)?;
                            break;
                        }
                        node = *next;
                    }
                    IfChainEntry::Else { units } => {
                        emit_units(units)?;
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
    ) -> Result<Vec<MaybeWithRange<&'a Token<'a>>>, ControlFlowErrorWithRange> {
        // Generate or use the provided value, validating RNG output range.
        let value_wr = self.value.into_wrapped_or(0..0);
        let value_range = value_wr.range().clone();
        let generated = match value_wr.into_content() {
            ControlFlowValue::GenMax(max) => {
                let range: RangeInclusive<BigUint> = BigUint::from(1u64)..=max;
                let g = rng.generate(range.clone());
                range.contains(&g).then_some(g.clone()).ok_or_else(|| {
                    SourceRangeMixin::new(
                        ControlFlowError::SwitchGeneratedValueOutOfRange {
                            expected: range,
                            actual: g,
                        },
                        value_range.clone(),
                    )
                })?
            }
            ControlFlowValue::Set(val) => val,
        };

        // Determine target case index: prefer default-before-first-case,
        // otherwise the matched case (with fallthrough), else last default.
        let cases = self.cases;
        let mut matched_index: Option<usize> = None;
        let mut last_default_index: Option<usize> = None;
        let mut seen_case = false;
        let mut first_default_before_any_case: Option<usize> = None;

        for (idx, case) in cases.iter().enumerate() {
            match case.condition {
                Some(ref cond) => {
                    seen_case = true;
                    if *cond.content() == generated {
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

        // Helper to activate nested units and append results.
        let mut out: Vec<MaybeWithRange<&'a Token<'a>>> = Vec::new();
        let mut emit_units = |units: Vec<TokenUnit<'a>>| -> Result<(), ControlFlowErrorWithRange> {
            for u in units {
                let tokens = Activate::activate(u, rng)?;
                out.extend(tokens);
            }
            Ok(())
        };

        // Execute selected case(s), respecting SKIP-based fallthrough stop.
        if let Some(start_idx) = target_index {
            let iter = cases.into_iter().skip(start_idx);

            if is_fallthrough {
                for case in iter {
                    emit_units(case.units)?;
                    if case.skip {
                        break;
                    }
                }
            } else if let Some(case) = iter.into_iter().next() {
                emit_units(case.units)?;
            }
        }

        Ok(out)
    }
}
