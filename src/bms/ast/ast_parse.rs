use num::BigUint;

use crate::bms::{command::mixin::SourcePosMixinExt, lex::token::TokenWithPos};
use core::ops::RangeInclusive;

use super::rng::Rng;
use super::{
    AstParseWarning, AstParseWarningWithPos,
    structure::{BlockValue, CaseBranchValue, Unit},
};

pub(super) fn parse_control_flow_ast<'a>(
    iter: &mut std::iter::Peekable<impl Iterator<Item = Unit<'a>>>,
    rng: &mut impl Rng,
) -> (Vec<&'a TokenWithPos<'a>>, Vec<AstParseWarningWithPos>) {
    let mut result: Vec<&'a TokenWithPos<'a>> = Vec::new();
    let mut warnings = Vec::new();
    for unit in iter.by_ref() {
        match unit {
            Unit::TokenWithPos(token) => {
                result.push(token);
            }
            Unit::RandomBlock {
                value, if_blocks, ..
            } => {
                // Select branch
                let (row, col) = (value.row(), value.column());
                let branch_val = match value.into_content() {
                    BlockValue::Random { max } if max == BigUint::from(0u64) => BigUint::from(0u64),
                    BlockValue::Random { max } => {
                        let expected: RangeInclusive<BigUint> = BigUint::from(1u64)..=max.clone();
                        let v = rng.generate(expected.clone());
                        if !expected.contains(&v) {
                            warnings.push(
                                AstParseWarning::RandomGeneratedValueOutOfRange {
                                    expected: expected.clone().into_wrapper_manual(row, col),
                                    actual: v.clone(),
                                }
                                .into_wrapper_manual(row, col),
                            );
                        }
                        v
                    }
                    BlockValue::Set { value } => value,
                };
                // Find the branch in the first if_block that contains this branch value
                let mut found = false;
                if let Some(branch) = if_blocks
                    .iter()
                    .flat_map(|if_block| if_block.branches.get(&branch_val))
                    .next()
                {
                    let mut branch_iter = branch.content().clone().into_iter().peekable();
                    let (tokens, inner_warnings) = parse_control_flow_ast(&mut branch_iter, rng);
                    result.extend(tokens);
                    warnings.extend(inner_warnings);
                    found = true;
                }
                // If not found, try to find the 0 (else) branch
                if !found
                    && let Some(else_branch) = if_blocks
                        .iter()
                        .flat_map(|if_block| if_block.branches.get(&BigUint::from(0u64)))
                        .next()
                {
                    let mut branch_iter = else_branch.content().clone().into_iter().peekable();
                    let (tokens, inner_warnings) = parse_control_flow_ast(&mut branch_iter, rng);
                    result.extend(tokens);
                    warnings.extend(inner_warnings);
                }
                // If none found, do nothing
            }
            Unit::SwitchBlock { value, cases, .. } => {
                let (row, col) = (value.row(), value.column());
                let switch_val = match value.into_content() {
                    BlockValue::Random { max } if max == BigUint::from(0u64) => BigUint::from(0u64),
                    BlockValue::Random { max } => {
                        let expected: RangeInclusive<BigUint> = BigUint::from(1u64)..=max.clone();
                        let v = rng.generate(expected.clone());
                        if !expected.contains(&v) {
                            warnings.push(
                                AstParseWarning::SwitchGeneratedValueOutOfRange {
                                    expected: expected.clone().into_wrapper_manual(row, col),
                                    actual: v.clone(),
                                }
                                .into_wrapper_manual(row, col),
                            );
                        }
                        v
                    }
                    BlockValue::Set { value } => value,
                };
                // Find Case branch
                let mut found = false;
                for case in &cases {
                    match case.value.content() {
                        CaseBranchValue::Case(val) if *val == switch_val => {
                            let mut case_iter = case.units.clone().into_iter().peekable();
                            let (tokens, inner_warnings) =
                                parse_control_flow_ast(&mut case_iter, rng);
                            result.extend(tokens);
                            warnings.extend(inner_warnings);
                            found = true;
                            break;
                        }
                        _ => {}
                    }
                }
                // If no Case matches, find the Def branch
                if !found {
                    for case in &cases {
                        if let CaseBranchValue::Def = case.value.content() {
                            let mut case_iter = case.units.clone().into_iter().peekable();
                            let (tokens, inner_warnings) =
                                parse_control_flow_ast(&mut case_iter, rng);
                            result.extend(tokens);
                            warnings.extend(inner_warnings);
                            break;
                        }
                    }
                }
            }
        }
    }
    (result, warnings)
}

#[cfg(test)]
mod tests {
    use core::ops::RangeInclusive;
    use std::collections::BTreeMap;

    use num::BigUint;

    use super::*;
    use crate::{
        ast::structure::{CaseBranch, CaseBranchValue, IfBlock, Unit},
        bms::lex::token::Token,
        command::mixin::{SourcePosMixin, SourcePosMixinExt},
    };

    struct DummyRng;
    impl Rng for DummyRng {
        fn generate(&mut self, range: RangeInclusive<BigUint>) -> BigUint {
            // Always return the maximum value
            range.end().clone()
        }
    }

    #[test]
    fn test_setrandom_setwitch_large_value() {
        use Token::*;
        // If/Case value is very large under SetRandom/SetSwitch
        let t_if = Title("LARGE_IF").into_wrapper_manual(0, 0);
        let t_case = Title("LARGE_CASE").into_wrapper_manual(0, 0);
        let mut if_branches = BTreeMap::new();
        if_branches.insert(
            BigUint::from(u64::MAX),
            vec![Unit::TokenWithPos(&t_if)].into_wrapper_manual(14, 23),
        );
        let units = vec![
            Unit::RandomBlock {
                value: BlockValue::Set {
                    value: BigUint::from(u64::MAX),
                }
                .into_wrapper_manual(14, 23),
                if_blocks: vec![IfBlock {
                    branches: if_branches.clone(),
                    end_if: None,
                }],
            },
            Unit::SwitchBlock {
                value: BlockValue::Set {
                    value: BigUint::from(u64::MAX),
                }
                .into_wrapper_manual(14, 23),
                cases: vec![CaseBranch {
                    value: CaseBranchValue::Case(BigUint::from(u64::MAX))
                        .into_wrapper_manual(14, 23),
                    units: vec![Unit::TokenWithPos(&t_case)],
                }],
                end_sw: ().into_wrapper_manual(14, 23),
            },
        ];
        let mut rng = DummyRng;
        let mut iter = units.into_iter().peekable();
        let (tokens, _warnings) = parse_control_flow_ast(&mut iter, &mut rng);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t.content() {
                Title(s) => Some(s),
                _ => None,
            })
            .collect();
        assert!(titles.iter().any(|s| **s == "LARGE_IF"));
        assert!(titles.iter().any(|s| **s == "LARGE_CASE"));
    }

    #[test]
    fn test_nested_random_switch() {
        use Token::*;
        // Nested Random and Switch, mutually nested
        let mut rng = DummyRng;
        // Random outer, Switch inner
        let t_switch_in_random = Title("SWITCH_IN_RANDOM").into_wrapper_manual(0, 0);
        let mut if_branches: BTreeMap<BigUint, SourcePosMixin<Vec<Unit<'_>>>> = BTreeMap::new();
        if_branches.insert(
            BigUint::from(1u64),
            vec![Unit::SwitchBlock {
                value: BlockValue::Set {
                    value: BigUint::from(2u64),
                }
                .into_wrapper_manual(14, 23),
                cases: vec![CaseBranch {
                    value: CaseBranchValue::Case(BigUint::from(2u64)).into_wrapper_manual(14, 23),
                    units: vec![Unit::TokenWithPos(&t_switch_in_random)],
                }],
                end_sw: ().into_wrapper_manual(14, 23),
            }]
            .into_wrapper_manual(14, 23),
        );
        let units = vec![Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u64),
            }
            .into_wrapper_manual(14, 23),
            if_blocks: vec![IfBlock {
                branches: if_branches,
                end_if: None,
            }],
        }];
        let mut iter = units.into_iter().peekable();
        let (tokens, _warnings) = parse_control_flow_ast(&mut iter, &mut rng);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t.content() {
                Title(s) => Some(s),
                _ => None,
            })
            .collect();
        assert!(titles.iter().any(|s| **s == "SWITCH_IN_RANDOM"));

        // Switch outer, Random inner
        let t_random_in_switch = Title("RANDOM_IN_SWITCH").into_wrapper_manual(0, 0);
        let cases = vec![CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u64)).into_wrapper_manual(14, 23),
            units: vec![Unit::RandomBlock {
                value: BlockValue::Set {
                    value: BigUint::from(2u64),
                }
                .into_wrapper_manual(14, 23),
                if_blocks: vec![{
                    let mut b = BTreeMap::new();
                    b.insert(
                        BigUint::from(2u64),
                        vec![Unit::TokenWithPos(&t_random_in_switch)].into_wrapper_manual(14, 23),
                    );
                    IfBlock {
                        branches: b,
                        end_if: None,
                    }
                }],
            }],
        }];
        let units2 = vec![Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u64),
            }
            .into_wrapper_manual(14, 23),
            cases,
            end_sw: ().into_wrapper_manual(14, 23),
        }];
        let mut iter2 = units2.into_iter().peekable();
        let (tokens2, _warnings) = parse_control_flow_ast(&mut iter2, &mut rng);
        let titles2: Vec<_> = tokens2
            .iter()
            .filter_map(|t| match t.content() {
                Title(s) => Some(s),
                _ => None,
            })
            .collect();
        assert!(titles2.iter().any(|s| **s == "RANDOM_IN_SWITCH"));
    }

    #[test]
    fn test_deeply_nested_random_switch() {
        use Token::*;
        // Deeply nested Random and Switch
        let mut rng = DummyRng;
        let t_deep_nested = Title("DEEP_NESTED").into_wrapper_manual(0, 0);
        let mut if_branches: BTreeMap<BigUint, SourcePosMixin<Vec<Unit<'_>>>> = BTreeMap::new();
        if_branches.insert(
            BigUint::from(1u64),
            vec![Unit::SwitchBlock {
                value: BlockValue::Set {
                    value: BigUint::from(1u64),
                }
                .into_wrapper_manual(14, 23),
                cases: vec![CaseBranch {
                    value: CaseBranchValue::Case(BigUint::from(1u64)).into_wrapper_manual(14, 23),
                    units: vec![Unit::RandomBlock {
                        value: BlockValue::Set {
                            value: BigUint::from(1u64),
                        }
                        .into_wrapper_manual(14, 23),
                        if_blocks: vec![{
                            let mut b = BTreeMap::new();
                            b.insert(
                                BigUint::from(1u64),
                                vec![Unit::TokenWithPos(&t_deep_nested)]
                                    .into_wrapper_manual(14, 23),
                            );
                            IfBlock {
                                branches: b,
                                end_if: None,
                            }
                        }],
                    }],
                }],
                end_sw: ().into_wrapper_manual(14, 23),
            }]
            .into_wrapper_manual(14, 23),
        );

        let units = vec![Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u64),
            }
            .into_wrapper_manual(14, 23),
            if_blocks: vec![IfBlock {
                branches: if_branches,
                end_if: None,
            }],
        }];
        let mut iter = units.into_iter().peekable();
        let (tokens, _warnings) = parse_control_flow_ast(&mut iter, &mut rng);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t.content() {
                Title(s) => Some(s),
                _ => None,
            })
            .collect();
        assert!(titles.iter().any(|s| **s == "DEEP_NESTED"));
    }
}
