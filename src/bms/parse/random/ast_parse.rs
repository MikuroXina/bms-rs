use crate::bms::{lex::token::Token, parse::rng::Rng};

use super::ast_build::*;

pub(super) fn parse_control_flow_ast<'a>(
    iter: &mut std::iter::Peekable<impl Iterator<Item = Unit<'a>>>,
    rng: &mut impl Rng,
) -> Vec<&'a Token<'a>> {
    let mut result = Vec::new();
    for unit in iter.by_ref() {
        match unit {
            Unit::Token(token) => {
                result.push(token);
            }
            Unit::RandomBlock { value, if_blocks } => {
                // Select branch
                let branch_val = match value {
                    BlockValue::Random { max } => {
                        if max == 0 {
                            0
                        } else {
                            rng.generate(1..=(max as u32)) as u64
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
                    result.extend(parse_control_flow_ast(&mut branch_iter, rng));
                    found = true;
                }
                // If not found, try to find the 0 (else) branch
                if !found {
                    if let Some(else_branch) = if_blocks
                        .iter()
                        .flat_map(|if_block| if_block.branches.get(&0))
                        .next()
                    {
                        let mut branch_iter = else_branch.tokens.clone().into_iter().peekable();
                        result.extend(parse_control_flow_ast(&mut branch_iter, rng));
                    }
                }
                // If none found, do nothing
            }
            Unit::SwitchBlock { value, cases } => {
                let switch_val = match value {
                    BlockValue::Random { max } => {
                        if max == 0 {
                            0
                        } else {
                            rng.generate(1..=(max as u32)) as u64
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
                            result.extend(parse_control_flow_ast(&mut case_iter, rng));
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
                            result.extend(parse_control_flow_ast(&mut case_iter, rng));
                            break;
                        }
                    }
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bms::lex::token::Token;
    use std::collections::HashMap;

    struct DummyRng;
    impl Rng for DummyRng {
        fn generate(&mut self, _range: std::ops::RangeInclusive<u32>) -> u32 {
            // Always return the maximum value
            *_range.end()
        }
    }

    #[test]
    fn test_setrandom_setwitch_large_value() {
        // If/Case value is very large under SetRandom/SetSwitch
        let t_if = Token::Title("LARGE_IF");
        let t_case = Token::Title("LARGE_CASE");
        let mut if_branches = HashMap::new();
        if_branches.insert(
            u64::MAX,
            IfBranch {
                value: u64::MAX,
                tokens: vec![Unit::Token(&t_if)],
            },
        );
        let units = vec![
            Unit::RandomBlock {
                value: BlockValue::Set { value: u64::MAX },
                if_blocks: vec![IfBlock {
                    branches: if_branches.clone(),
                }],
            },
            Unit::SwitchBlock {
                value: BlockValue::Set { value: u64::MAX },
                cases: vec![CaseBranch {
                    value: CaseBranchValue::Case(u64::MAX),
                    tokens: vec![Unit::Token(&t_case)],
                }],
            },
        ];
        let mut rng = DummyRng;
        let mut iter = units.into_iter().peekable();
        let tokens = parse_control_flow_ast(&mut iter, &mut rng);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t {
                Token::Title(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert!(titles.contains(&"LARGE_IF"));
        assert!(titles.contains(&"LARGE_CASE"));
    }

    #[test]
    fn test_nested_random_switch() {
        // Nested Random and Switch, mutually nested
        let mut rng = DummyRng;
        let mut errors = Vec::new();
        // Random outer, Switch inner
        let t_switch_in_random = Token::Title("SWITCH_IN_RANDOM");
        let mut if_branches = HashMap::new();
        if_branches.insert(
            1,
            IfBranch {
                value: 1,
                tokens: vec![Unit::SwitchBlock {
                    value: BlockValue::Set { value: 2 },
                    cases: vec![CaseBranch {
                        value: CaseBranchValue::Case(2),
                        tokens: vec![Unit::Token(&t_switch_in_random)],
                    }],
                }],
            },
        );
        let units = vec![Unit::RandomBlock {
            value: BlockValue::Set { value: 1 },
            if_blocks: vec![IfBlock {
                branches: if_branches,
            }],
        }];
        let mut iter = units.into_iter().peekable();
        let tokens = parse_control_flow_ast(&mut iter, &mut rng, &mut errors);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t {
                Token::Title(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert!(titles.contains(&"SWITCH_IN_RANDOM"));

        // Switch outer, Random inner
        let t_random_in_switch = Token::Title("RANDOM_IN_SWITCH");
        let cases = vec![CaseBranch {
            value: CaseBranchValue::Case(1),
            tokens: vec![Unit::RandomBlock {
                value: BlockValue::Set { value: 2 },
                if_blocks: vec![{
                    let mut b = HashMap::new();
                    b.insert(
                        2,
                        IfBranch {
                            value: 2,
                            tokens: vec![Unit::Token(&t_random_in_switch)],
                        },
                    );
                    IfBlock { branches: b }
                }],
            }],
        }];
        let units2 = vec![Unit::SwitchBlock {
            value: BlockValue::Set { value: 1 },
            cases,
        }];
        let mut iter2 = units2.into_iter().peekable();
        let tokens2 = parse_control_flow_ast(&mut iter2, &mut rng, &mut errors);
        let titles2: Vec<_> = tokens2
            .iter()
            .filter_map(|t| match t {
                Token::Title(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert!(titles2.contains(&"RANDOM_IN_SWITCH"));
    }

    #[test]
    fn test_deeply_nested_random_switch() {
        // Deeply nested Random and Switch
        let mut rng = DummyRng;
        let mut errors = Vec::new();
        let t_deep_nested = Token::Title("DEEP_NESTED");
        let mut if_branches = HashMap::new();
        if_branches.insert(
            1,
            IfBranch {
                value: 1,
                tokens: vec![Unit::SwitchBlock {
                    value: BlockValue::Set { value: 1 },
                    cases: vec![CaseBranch {
                        value: CaseBranchValue::Case(1),
                        tokens: vec![Unit::RandomBlock {
                            value: BlockValue::Set { value: 1 },
                            if_blocks: vec![{
                                let mut b = HashMap::new();
                                b.insert(
                                    1,
                                    IfBranch {
                                        value: 1,
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
            value: BlockValue::Set { value: 1 },
            if_blocks: vec![IfBlock {
                branches: if_branches,
            }],
        }];
        let mut iter = units.into_iter().peekable();
        let tokens = parse_control_flow_ast(&mut iter, &mut rng, &mut errors);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t {
                Token::Title(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert!(titles.contains(&"DEEP_NESTED"));
    }
}
