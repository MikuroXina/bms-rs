//! The module for parsing the control flow AST.

use num::BigUint;

use crate::bms::lex::token::Token;

use super::{
    rng::Rng,
    structure::{AstParseWarning, AstParseWarningType, BlockValue, CaseBranchValue, IfBlock, Unit},
};

/// Parse [`Unit`] list into activated token list.
pub fn parse_control_flow_ast<'a>(
    iter: &mut std::iter::Peekable<impl Iterator<Item = Unit<'a>>>,
    rng: &mut impl Rng,
) -> (Vec<&'a Token<'a>>, Vec<AstParseWarning>) {
    let mut result = Vec::new();
    let mut warnings: Vec<AstParseWarning> = Vec::new();
    for unit in iter.by_ref() {
        match unit {
            Unit::Token(token) => {
                result.push(token);
            }
            Unit::RandomBlock { value, if_blocks } => {
                // Range validation moved from build phase to parse phase
                if let BlockValue::Random { max } = &value {
                    validate_random_ifblocks_ranges(&mut warnings, &if_blocks, max);
                }
                // Select branch
                let branch_val = match value {
                    BlockValue::Random { max } => {
                        if max == BigUint::from(0u64) {
                            BigUint::from(0u64)
                        } else {
                            rng.generate(BigUint::from(1u64)..=max)
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
                    let (tokens, mut inner_warnings) =
                        parse_control_flow_ast(&mut branch_iter, rng);
                    result.extend(tokens);
                    warnings.append(&mut inner_warnings);
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
                    let (tokens, mut inner_warnings) =
                        parse_control_flow_ast(&mut branch_iter, rng);
                    result.extend(tokens);
                    warnings.append(&mut inner_warnings);
                }
                // If none found, do nothing
            }
            Unit::SwitchBlock { value, cases } => {
                // Range validation moved from build phase to parse phase
                if let BlockValue::Random { max } = &value {
                    let max_owned = max.clone();
                    for case in &cases {
                        if let CaseBranchValue::Case(ref val) = case.value
                            && !(BigUint::from(1u64)..=max_owned.clone()).contains(val)
                        {
                            warnings.push(
                                AstParseWarningType::SwitchCaseValueOutOfRange
                                    .to_parse_warning_manual(case.row, case.col),
                            );
                        }
                    }
                }
                let switch_val = match value {
                    BlockValue::Random { max } => {
                        if max == BigUint::from(0u64) {
                            BigUint::from(0u64)
                        } else {
                            rng.generate(BigUint::from(1u64)..=max)
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
                            let (tokens, mut inner_warnings) =
                                parse_control_flow_ast(&mut case_iter, rng);
                            result.extend(tokens);
                            warnings.append(&mut inner_warnings);
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
                            let (tokens, mut inner_warnings) =
                                parse_control_flow_ast(&mut case_iter, rng);
                            result.extend(tokens);
                            warnings.append(&mut inner_warnings);
                            break;
                        }
                    }
                }
            }
        }
    }
    (result, warnings)
}

fn validate_random_ifblocks_ranges(
    warnings: &mut Vec<AstParseWarning>,
    if_blocks: &Vec<IfBlock<'_>>,
    max: &BigUint,
) {
    let max_owned = max.clone();
    for if_block in if_blocks {
        for if_branch in if_block.branches.values() {
            // 0 is Else branch and is allowed always
            if if_branch.value == BigUint::from(0u64) {
                continue;
            }
            if !(BigUint::from(1u64)..=max_owned.clone()).contains(&if_branch.value) {
                warnings.push(
                    AstParseWarningType::RandomIfBranchValueOutOfRange
                        .to_parse_warning_manual(if_branch.row, if_branch.col),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::ops::RangeInclusive;
    use std::collections::HashMap;

    use num::BigUint;

    use super::*;
    use crate::bms::{
        ast::structure::{CaseBranch, IfBlock, IfBranch},
        lex::token::{Token, TokenContent},
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
        use TokenContent::*;
        // If/Case value is very large under SetRandom/SetSwitch
        let t_if = Token {
            content: Title("LARGE_IF"),
            row: 0,
            col: 0,
        };
        let t_case = Token {
            content: Title("LARGE_CASE"),
            row: 0,
            col: 0,
        };
        let mut if_branches = HashMap::new();
        if_branches.insert(
            BigUint::from(u64::MAX),
            IfBranch {
                value: BigUint::from(u64::MAX),
                row: 0,
                col: 0,
                tokens: vec![Unit::Token(&t_if)],
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
                    row: 0,
                    col: 0,
                    tokens: vec![Unit::Token(&t_case)],
                }],
            },
        ];
        let mut rng = DummyRng;
        let mut iter = units.into_iter().peekable();
        let (tokens, _w) = parse_control_flow_ast(&mut iter, &mut rng);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t {
                Token {
                    content: Title(s), ..
                } => Some(*s),
                _ => None,
            })
            .collect();
        assert!(titles.contains(&"LARGE_IF"));
        assert!(titles.contains(&"LARGE_CASE"));
    }

    #[test]
    fn test_nested_random_switch() {
        use TokenContent::*;
        // Nested Random and Switch, mutually nested
        let mut rng = DummyRng;
        // Random outer, Switch inner
        let t_switch_in_random = Token {
            content: Title("SWITCH_IN_RANDOM"),
            row: 0,
            col: 0,
        };
        let mut if_branches = HashMap::new();
        if_branches.insert(
            BigUint::from(1u64),
            IfBranch {
                value: BigUint::from(1u64),
                row: 0,
                col: 0,
                tokens: vec![Unit::SwitchBlock {
                    value: BlockValue::Set {
                        value: BigUint::from(2u64),
                    },
                    cases: vec![CaseBranch {
                        value: CaseBranchValue::Case(BigUint::from(2u64)),
                        row: 0,
                        col: 0,
                        tokens: vec![Unit::Token(&t_switch_in_random)],
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
        let (tokens, _w) = parse_control_flow_ast(&mut iter, &mut rng);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t {
                Token {
                    content: Title(s), ..
                } => Some(*s),
                _ => None,
            })
            .collect();
        assert!(titles.contains(&"SWITCH_IN_RANDOM"));

        // Switch outer, Random inner
        let t_random_in_switch = Token {
            content: Title("RANDOM_IN_SWITCH"),
            row: 0,
            col: 0,
        };
        let cases = vec![CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u64)),
            row: 0,
            col: 0,
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
                            row: 0,
                            col: 0,
                            tokens: vec![Unit::Token(&t_random_in_switch)],
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
        let (tokens2, _w) = parse_control_flow_ast(&mut iter2, &mut rng);
        let titles2: Vec<_> = tokens2
            .iter()
            .filter_map(|t| match t {
                Token {
                    content: Title(s), ..
                } => Some(*s),
                _ => None,
            })
            .collect();
        assert!(titles2.contains(&"RANDOM_IN_SWITCH"));
    }

    #[test]
    fn test_deeply_nested_random_switch() {
        use TokenContent::*;
        // Deeply nested Random and Switch
        let mut rng = DummyRng;
        let t_deep_nested = Token {
            content: Title("DEEP_NESTED"),
            row: 0,
            col: 0,
        };
        let mut if_branches = HashMap::new();
        if_branches.insert(
            BigUint::from(1u64),
            IfBranch {
                value: BigUint::from(1u64),
                row: 0,
                col: 0,
                tokens: vec![Unit::SwitchBlock {
                    value: BlockValue::Set {
                        value: BigUint::from(1u64),
                    },
                    cases: vec![CaseBranch {
                        value: CaseBranchValue::Case(BigUint::from(1u64)),
                        row: 0,
                        col: 0,
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
                                        row: 0,
                                        col: 0,
                                        tokens: vec![Unit::Token(&t_deep_nested)],
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
        let (tokens, _w) = parse_control_flow_ast(&mut iter, &mut rng);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t {
                Token {
                    content: Title(s), ..
                } => Some(*s),
                _ => None,
            })
            .collect();
        assert!(titles.contains(&"DEEP_NESTED"));
    }
}
