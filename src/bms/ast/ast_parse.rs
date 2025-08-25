use num::BigUint;

use crate::bms::command::mixin::SourcePosMixinExt;
use crate::bms::lex::token::TokenWithPos;

use super::ast_build::*;
use super::rng::Rng;
use super::{AstParseWarning, AstParseWarningWithPos};

pub(super) fn parse_control_flow_ast<'a>(
    iter: &mut std::iter::Peekable<impl Iterator<Item = Unit<'a>>>,
    rng: &mut impl Rng,
) -> (Vec<&'a TokenWithPos<'a>>, Vec<AstParseWarningWithPos>) {
    let mut result = Vec::new();
    let mut warnings = Vec::new();
    for unit in iter.by_ref() {
        match unit {
            Unit::TokenWithPos(token) => {
                result.push(token);
            }
            Unit::RandomBlock { value, if_blocks } => {
                // Select branch
                let branch_val = match value {
                    BlockValue::Random { max } => {
                        if max == BigUint::from(0u64) {
                            BigUint::from(0u64)
                        } else {
                            let expected_range = BigUint::from(1u64)..=max.clone();
                            let generated = rng.generate(expected_range.clone());
                            // Check if generated value is within expected range
                            if generated < BigUint::from(1u64) || generated > max {
                                warnings.push(
                                    AstParseWarning::RandomValueOutOfRange {
                                        expected_range,
                                        actual_value: generated.clone(),
                                    }
                                    .into_wrapper_manual(0, 0),
                                );
                            }
                            generated
                        }
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
                    let mut branch_iter = branch.tokens.clone().into_iter().peekable();
                    let (branch_tokens, branch_warnings) =
                        parse_control_flow_ast(&mut branch_iter, rng);
                    result.extend(branch_tokens);
                    warnings.extend(branch_warnings);
                    found = true;
                }
                // If not found, try to find the 0 (else) branch
                if !found
                    && let Some(else_branch) = if_blocks
                        .iter()
                        .flat_map(|if_block| if_block.branches.get(&BigUint::from(0u64)))
                        .next()
                {
                    let mut branch_iter = else_branch.tokens.clone().into_iter().peekable();
                    let (branch_tokens, branch_warnings) =
                        parse_control_flow_ast(&mut branch_iter, rng);
                    result.extend(branch_tokens);
                    warnings.extend(branch_warnings);
                }
                // If no matching branch found, do nothing (this is normal behavior)
            }
            Unit::SwitchBlock { value, cases } => {
                let switch_val = match value {
                    BlockValue::Random { max } => {
                        if max == BigUint::from(0u64) {
                            BigUint::from(0u64)
                        } else {
                            let expected_range = BigUint::from(1u64)..=max.clone();
                            let generated = rng.generate(expected_range.clone());
                            // Check if generated value is within expected range
                            if generated < BigUint::from(1u64) || generated > max {
                                warnings.push(
                                    AstParseWarning::SwitchValueOutOfRange {
                                        expected_range,
                                        actual_value: generated.clone(),
                                    }
                                    .into_wrapper_manual(0, 0),
                                );
                            }
                            generated
                        }
                    }
                    BlockValue::Set { value } => value,
                };
                // Find Case branch
                let mut found = false;
                for case in &cases {
                    match &case.value {
                        CaseBranchValue::Case(val) if *val == switch_val => {
                            let mut case_iter = case.tokens.clone().into_iter().peekable();
                            let (case_tokens, case_warnings) =
                                parse_control_flow_ast(&mut case_iter, rng);
                            result.extend(case_tokens);
                            warnings.extend(case_warnings);
                            found = true;
                            break;
                        }
                        _ => {}
                    }
                }
                // If no Case matches, find the Def branch
                if !found {
                    for case in &cases {
                        if let CaseBranchValue::Def = case.value {
                            let mut case_iter = case.tokens.clone().into_iter().peekable();
                            let (case_tokens, case_warnings) =
                                parse_control_flow_ast(&mut case_iter, rng);
                            result.extend(case_tokens);
                            warnings.extend(case_warnings);
                            break;
                        }
                    }
                }
                // If no matching case found, do nothing (this is normal behavior)
            }
        }
    }
    (result, warnings)
}

#[cfg(test)]
mod tests {
    use core::ops::RangeInclusive;
    use std::collections::HashMap;

    use num::BigUint;

    use super::*;
    use crate::{bms::lex::token::Token, command::mixin::SourcePosMixinExt};

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
        let mut if_branches = HashMap::new();
        if_branches.insert(
            BigUint::from(u64::MAX),
            IfBranch {
                value: BigUint::from(u64::MAX),
                tokens: vec![Unit::TokenWithPos(&t_if)],
            },
        );
        let units = vec![
            Unit::RandomBlock {
                value: BlockValue::Set {
                    value: BigUint::from(u64::MAX),
                },
                if_blocks: vec![IfBlock {
                    branches: if_branches.clone(),
                }],
            },
            Unit::SwitchBlock {
                value: BlockValue::Set {
                    value: BigUint::from(u64::MAX),
                },
                cases: vec![CaseBranch {
                    value: CaseBranchValue::Case(BigUint::from(u64::MAX)),
                    tokens: vec![Unit::TokenWithPos(&t_case)],
                }],
            },
        ];
        let mut rng = DummyRng;
        let mut iter = units.into_iter().peekable();
        let (tokens, _warnings) = parse_control_flow_ast(&mut iter, &mut rng);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t.content() {
                Title(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert!(titles.contains(&"LARGE_IF"));
        assert!(titles.contains(&"LARGE_CASE"));
    }

    #[test]
    fn test_nested_random_switch() {
        use Token::*;
        // Nested Random and Switch, mutually nested
        let mut rng = DummyRng;
        // Random outer, Switch inner
        let t_switch_in_random = Title("SWITCH_IN_RANDOM").into_wrapper_manual(0, 0);
        let mut if_branches = HashMap::new();
        if_branches.insert(
            BigUint::from(1u64),
            IfBranch {
                value: BigUint::from(1u64),
                tokens: vec![Unit::SwitchBlock {
                    value: BlockValue::Set {
                        value: BigUint::from(2u64),
                    },
                    cases: vec![CaseBranch {
                        value: CaseBranchValue::Case(BigUint::from(2u64)),
                        tokens: vec![Unit::TokenWithPos(&t_switch_in_random)],
                    }],
                }],
            },
        );
        let units = vec![Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u64),
            },
            if_blocks: vec![IfBlock {
                branches: if_branches,
            }],
        }];
        let mut iter = units.into_iter().peekable();
        let (tokens, _warnings) = parse_control_flow_ast(&mut iter, &mut rng);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t.content() {
                Title(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert!(titles.contains(&"SWITCH_IN_RANDOM"));

        // Switch outer, Random inner
        let t_random_in_switch = Title("RANDOM_IN_SWITCH").into_wrapper_manual(0, 0);
        let cases = vec![CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u64)),
            tokens: vec![Unit::RandomBlock {
                value: BlockValue::Set {
                    value: BigUint::from(2u64),
                },
                if_blocks: vec![{
                    let mut b = HashMap::new();
                    b.insert(
                        BigUint::from(2u64),
                        IfBranch {
                            value: BigUint::from(2u64),
                            tokens: vec![Unit::TokenWithPos(&t_random_in_switch)],
                        },
                    );
                    IfBlock { branches: b }
                }],
            }],
        }];
        let units2 = vec![Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u64),
            },
            cases,
        }];
        let mut iter2 = units2.into_iter().peekable();
        let (tokens2, _warnings2) = parse_control_flow_ast(&mut iter2, &mut rng);
        let titles2: Vec<_> = tokens2
            .iter()
            .filter_map(|t| match t.content() {
                Title(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert!(titles2.contains(&"RANDOM_IN_SWITCH"));
    }

    #[test]
    fn test_deeply_nested_random_switch() {
        use Token::*;
        // Deeply nested Random and Switch
        let mut rng = DummyRng;
        let t_deep_nested = Title("DEEP_NESTED").into_wrapper_manual(0, 0);
        let mut if_branches = HashMap::new();
        if_branches.insert(
            BigUint::from(1u64),
            IfBranch {
                value: BigUint::from(1u64),
                tokens: vec![Unit::SwitchBlock {
                    value: BlockValue::Set {
                        value: BigUint::from(1u64),
                    },
                    cases: vec![CaseBranch {
                        value: CaseBranchValue::Case(BigUint::from(1u64)),
                        tokens: vec![Unit::RandomBlock {
                            value: BlockValue::Set {
                                value: BigUint::from(1u64),
                            },
                            if_blocks: vec![{
                                let mut b = HashMap::new();
                                b.insert(
                                    BigUint::from(1u64),
                                    IfBranch {
                                        value: BigUint::from(1u64),
                                        tokens: vec![Unit::TokenWithPos(&t_deep_nested)],
                                    },
                                );
                                IfBlock { branches: b }
                            }],
                        }],
                    }],
                }],
            },
        );
        let units = vec![Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u64),
            },
            if_blocks: vec![IfBlock {
                branches: if_branches,
            }],
        }];
        let mut iter = units.into_iter().peekable();
        let (tokens, _warnings) = parse_control_flow_ast(&mut iter, &mut rng);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t.content() {
                Title(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert!(titles.contains(&"DEEP_NESTED"));
    }
}
