//! Activation module for control-flow models.
//!
//! This module provides a trait to evaluate `Random`/`Switch` control-flow
//! structures using an `Rng` and produce activated tokens with source ranges.

use std::ops::RangeInclusive;

use num::BigUint;

use crate::bms::lex::token::{Token, TokenWithRange};

use crate::bms::rng::Rng;

use crate::bms::command::mixin::SourceRangeMixin;
use crate::bms::parse::{ControlFlowError, ControlFlowErrorWithRange, ParseWarningWithRange};

use super::{ControlFlowValue, IfChainEntry, Random, Switch, TokenUnit};

/// Activates control-flow structures using an RNG, returning tokens and warnings.
pub trait Activate<'a> {
    /// Evaluate control-flow and return `(tokens, warnings)` or an error.
    fn activate<R: Rng>(
        self,
        rng: &mut R,
    ) -> Result<(Vec<TokenWithRange<'a>>, Vec<ParseWarningWithRange>), ControlFlowErrorWithRange>;
}

impl<'a> Activate<'a> for TokenUnit<'a> {
    fn activate<R: Rng>(
        self,
        rng: &mut R,
    ) -> Result<(Vec<TokenWithRange<'a>>, Vec<ParseWarningWithRange>), ControlFlowErrorWithRange>
    {
        match self {
            TokenUnit::Random(r) => Activate::activate(r, rng),
            TokenUnit::Switch(s) => Activate::activate(s, rng),
            TokenUnit::Tokens(v) => Ok((
                v.into_iter()
                    .map(Token::from)
                    .map(|t| SourceRangeMixin::new(t, 0..0))
                    .collect(),
                Vec::new(),
            )),
        }
    }
}

impl<'a> Activate<'a> for Random<'a> {
    fn activate<R: Rng>(
        self,
        rng: &mut R,
    ) -> Result<(Vec<TokenWithRange<'a>>, Vec<ParseWarningWithRange>), ControlFlowErrorWithRange>
    {
        let generated = match self.value {
            ControlFlowValue::GenMax(max) => {
                let range: RangeInclusive<BigUint> = BigUint::from(1u64)..=max;
                let g = rng.generate(range.clone());
                if !range.contains(&g) {
                    return Err(SourceRangeMixin::new(
                        ControlFlowError::RandomGeneratedValueOutOfRange {
                            expected: range,
                            actual: g,
                        },
                        0..0,
                    ));
                }
                g
            }
            ControlFlowValue::Set(val) => val,
        };

        let mut out: Vec<TokenWithRange<'a>> = Vec::new();
        for branch in self.branches.into_iter() {
            if branch.condition == generated {
                for u in branch.head_units.into_iter() {
                    let (tokens, _warns) = Activate::activate(u, rng)?;
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
                                let (tokens, _warns) = Activate::activate(u, rng)?;
                                out.extend(tokens);
                            }
                            break;
                        }
                        node = *next;
                    }
                    IfChainEntry::Else { units } => {
                        for u in units.into_iter() {
                            let (tokens, _warns) = Activate::activate(u, rng)?;
                            out.extend(tokens);
                        }
                        break;
                    }
                    IfChainEntry::EndIf => break,
                }
            }
        }
        Ok((out, Vec::new()))
    }
}

impl<'a> Activate<'a> for Switch<'a> {
    fn activate<R: Rng>(
        self,
        rng: &mut R,
    ) -> Result<(Vec<TokenWithRange<'a>>, Vec<ParseWarningWithRange>), ControlFlowErrorWithRange>
    {
        let generated = match self.value {
            ControlFlowValue::GenMax(max) => {
                let range: RangeInclusive<BigUint> = BigUint::from(1u64)..=max;
                let g = rng.generate(range.clone());
                if !range.contains(&g) {
                    return Err(SourceRangeMixin::new(
                        ControlFlowError::SwitchGeneratedValueOutOfRange {
                            expected: range,
                            actual: g,
                        },
                        0..0,
                    ));
                }
                g
            }
            ControlFlowValue::Set(val) => val,
        };

        let mut out: Vec<TokenWithRange<'a>> = Vec::new();
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

        if let Some(i) = first_default_before_any_case {
            for u in cases[i].units.clone().into_iter() {
                let (tokens, _warns) = Activate::activate(u, rng)?;
                out.extend(tokens);
            }
            return Ok((out, Vec::new()));
        }

        if let Some(i) = matched_index {
            let mut j = i;
            while j < cases.len() {
                let case = &cases[j];
                for u in case.units.clone().into_iter() {
                    let (tokens, _warns) = Activate::activate(u, rng)?;
                    out.extend(tokens);
                }
                if case.skip {
                    break;
                }
                j += 1;
            }
            return Ok((out, Vec::new()));
        }

        if let Some(i) = last_default_index {
            let case = &cases[i];
            for u in case.units.clone().into_iter() {
                let (tokens, _warns) = Activate::activate(u, rng)?;
                out.extend(tokens);
            }
        }
        Ok((out, Vec::new()))
    }
}
