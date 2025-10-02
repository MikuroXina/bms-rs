use core::ops::RangeInclusive;

use ariadne::{Color, Label, Report, ReportKind};
use num::BigUint;

use crate::{
    bms::{command::mixin::SourceRangeMixinExt, lex::token::TokenWithRange},
    diagnostics::{SimpleSource, ToAriadne},
};

use super::{
    AstParseWarning, AstParseWarningWithRange,
    rng::Rng,
    structure::{BlockValue, CaseBranchValue, Unit},
};

pub(super) fn parse_control_flow_ast<'a>(
    iter: &mut std::iter::Peekable<impl Iterator<Item = Unit<'a>>>,
    rng: &mut impl Rng,
) -> (Vec<&'a TokenWithRange<'a>>, Vec<AstParseWarningWithRange>) {
    let mut result: Vec<&'a TokenWithRange<'a>> = Vec::new();
    let mut warnings = Vec::new();
    for unit in iter.by_ref() {
        match unit {
            Unit::TokenWithRange(token) => {
                result.push(token);
            }
            Unit::RandomBlock {
                value, if_blocks, ..
            } => {
                // Select branch
                let (start, end) = value.as_span();
                let branch_val = match value.into_content() {
                    BlockValue::Random { max } if max == BigUint::from(0u64) => BigUint::from(0u64),
                    BlockValue::Random { max } => {
                        let expected: RangeInclusive<BigUint> = BigUint::from(1u64)..=max.clone();
                        let v = rng.generate(expected.clone());
                        if !expected.contains(&v) {
                            warnings.push(
                                AstParseWarning::RandomGeneratedValueOutOfRange {
                                    expected: expected.clone().into_wrapper_range(start..end),
                                    actual: v.clone(),
                                }
                                .into_wrapper_range(start..end),
                            );
                        }
                        v
                    }
                    BlockValue::Set { value } => value,
                };
                if let Some(branch) = if_blocks
                    .iter()
                    .find_map(|if_block| if_block.branches.get(&branch_val))
                {
                    // Find the branch in the first if_block that contains this branch value
                    let mut branch_iter = branch.content().clone().into_iter().peekable();
                    let (tokens, inner_warnings) = parse_control_flow_ast(&mut branch_iter, rng);
                    result.extend(tokens);
                    warnings.extend(inner_warnings);
                } else if let Some(else_branch) = if_blocks
                    .iter()
                    .find_map(|if_block| if_block.branches.get(&BigUint::from(0u64)))
                {
                    // If not found, try to find the 0 (else) branch
                    let mut branch_iter = else_branch.content().clone().into_iter().peekable();
                    let (tokens, inner_warnings) = parse_control_flow_ast(&mut branch_iter, rng);
                    result.extend(tokens);
                    warnings.extend(inner_warnings);
                }
                // If none found, do nothing
            }
            Unit::SwitchBlock { value, cases, .. } => {
                let (start, end) = value.as_span();
                let switch_val = match value.into_content() {
                    BlockValue::Random { max } if max == BigUint::from(0u64) => BigUint::from(0u64),
                    BlockValue::Random { max } => {
                        let expected: RangeInclusive<BigUint> = BigUint::from(1u64)..=max.clone();
                        let v = rng.generate(expected.clone());
                        if !expected.contains(&v) {
                            warnings.push(
                                AstParseWarning::SwitchGeneratedValueOutOfRange {
                                    expected: expected.clone().into_wrapper_range(start..end),
                                    actual: v.clone(),
                                }
                                .into_wrapper_range(start..end),
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
                        if matches!(case.value.content(), CaseBranchValue::Def) {
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

impl ToAriadne for AstParseWarningWithRange {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        let (start, end) = self.as_span();

        // AstParseWarning internally has nested SourcePosMixin<RangeInclusive<BigUint>>, but it also has a top-level position.
        // We use the top-level position to annotate the error range and append expected/actual information to the message.
        let details = match self.content() {
            AstParseWarning::RandomGeneratedValueOutOfRange { expected, actual }
            | AstParseWarning::SwitchGeneratedValueOutOfRange { expected, actual } => {
                format!("expected {:?}, got {}", expected.content(), actual)
            }
        };
        let filename = src.name().to_string();
        Report::build(ReportKind::Warning, (filename.clone(), start..end))
            .with_message("ast_parse: ".to_string() + &self.content().to_string() + &details)
            .with_label(Label::new((filename, start..end)).with_color(Color::Cyan))
            .finish()
    }
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
        command::mixin::{SourceRangeMixin, SourceRangeMixinExt},
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
        // If/Case value is very large under SetRandom/SetSwitch
        let t_if = Token::header("TITLE", "LARGE_IF").into_wrapper_range(0..0);
        let t_case = Token::header("TITLE", "LARGE_CASE").into_wrapper_range(0..0);
        let mut if_branches = BTreeMap::new();
        if_branches.insert(
            BigUint::from(u64::MAX),
            vec![Unit::TokenWithRange(&t_if)].into_wrapper_range(14..23),
        );
        let units = vec![
            Unit::RandomBlock {
                value: BlockValue::Set {
                    value: BigUint::from(u64::MAX),
                }
                .into_wrapper_range(14..23),
                if_blocks: vec![IfBlock {
                    branches: if_branches.clone(),
                    end_if: ().into_wrapper_range(14..23),
                }],
                end_random: ().into_wrapper_range(14..23),
            },
            Unit::SwitchBlock {
                value: BlockValue::Set {
                    value: BigUint::from(u64::MAX),
                }
                .into_wrapper_range(14..23),
                cases: vec![CaseBranch {
                    value: CaseBranchValue::Case(BigUint::from(u64::MAX))
                        .into_wrapper_range(14..23),
                    units: vec![Unit::TokenWithRange(&t_case)],
                }],
                end_sw: ().into_wrapper_range(14..23),
            },
        ];
        let mut rng = DummyRng;
        let mut iter = units.into_iter().peekable();
        let (tokens, _warnings) = parse_control_flow_ast(&mut iter, &mut rng);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t.content() {
                Token::Header { name, args } if name == "TITLE" => Some(args),
                _ => None,
            })
            .collect();
        assert!(titles.iter().any(|s| **s == "LARGE_IF"));
        assert!(titles.iter().any(|s| **s == "LARGE_CASE"));
    }

    #[test]
    fn test_nested_random_switch() {
        // Nested Random and Switch, mutually nested
        let mut rng = DummyRng;
        // Random outer, Switch inner
        let t_switch_in_random =
            Token::header("TITLE", "SWITCH_IN_RANDOM").into_wrapper_range(0..0);
        let mut if_branches: BTreeMap<BigUint, SourceRangeMixin<Vec<Unit<'_>>>> = BTreeMap::new();
        if_branches.insert(
            BigUint::from(1u64),
            vec![Unit::SwitchBlock {
                value: BlockValue::Set {
                    value: BigUint::from(2u64),
                }
                .into_wrapper_range(14..23),
                cases: vec![CaseBranch {
                    value: CaseBranchValue::Case(BigUint::from(2u64)).into_wrapper_range(14..23),
                    units: vec![Unit::TokenWithRange(&t_switch_in_random)],
                }],
                end_sw: ().into_wrapper_range(14..23),
            }]
            .into_wrapper_range(14..23),
        );
        let units = vec![Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u64),
            }
            .into_wrapper_range(14..23),
            if_blocks: vec![IfBlock {
                branches: if_branches,
                end_if: ().into_wrapper_range(14..23),
            }],
            end_random: ().into_wrapper_range(14..23),
        }];
        let mut iter = units.into_iter().peekable();
        let (tokens, _warnings) = parse_control_flow_ast(&mut iter, &mut rng);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t.content() {
                Token::Header { name, args } if name == "TITLE" => Some(args),
                _ => None,
            })
            .collect();
        assert!(titles.iter().any(|s| **s == "SWITCH_IN_RANDOM"));

        // Switch outer, Random inner
        let t_random_in_switch =
            Token::header("TITLE", "RANDOM_IN_SWITCH").into_wrapper_range(0..0);
        let cases = vec![CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u64)).into_wrapper_range(14..23),
            units: vec![Unit::RandomBlock {
                value: BlockValue::Set {
                    value: BigUint::from(2u64),
                }
                .into_wrapper_range(14..23),
                if_blocks: vec![{
                    let mut b = BTreeMap::new();
                    b.insert(
                        BigUint::from(2u64),
                        vec![Unit::TokenWithRange(&t_random_in_switch)].into_wrapper_range(14..23),
                    );
                    IfBlock {
                        branches: b,
                        end_if: ().into_wrapper_range(14..23),
                    }
                }],
                end_random: ().into_wrapper_range(14..23),
            }],
        }];
        let units2 = vec![Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u64),
            }
            .into_wrapper_range(14..23),
            cases,
            end_sw: ().into_wrapper_range(14..23),
        }];
        let mut iter2 = units2.into_iter().peekable();
        let (tokens2, _warnings) = parse_control_flow_ast(&mut iter2, &mut rng);
        let titles2: Vec<_> = tokens2
            .iter()
            .filter_map(|t| match t.content() {
                Token::Header { name, args } if name == "TITLE" => Some(args),
                _ => None,
            })
            .collect();
        assert!(titles2.iter().any(|s| **s == "RANDOM_IN_SWITCH"));
    }

    #[test]
    fn test_deeply_nested_random_switch() {
        // Deeply nested Random and Switch
        let mut rng = DummyRng;
        let t_deep_nested = Token::header("TITLE", "DEEP_NESTED").into_wrapper_range(0..0);
        let mut if_branches: BTreeMap<BigUint, SourceRangeMixin<Vec<Unit<'_>>>> = BTreeMap::new();
        if_branches.insert(
            BigUint::from(1u64),
            vec![Unit::SwitchBlock {
                value: BlockValue::Set {
                    value: BigUint::from(1u64),
                }
                .into_wrapper_range(14..23),
                cases: vec![CaseBranch {
                    value: CaseBranchValue::Case(BigUint::from(1u64)).into_wrapper_range(14..23),
                    units: vec![Unit::RandomBlock {
                        value: BlockValue::Set {
                            value: BigUint::from(1u64),
                        }
                        .into_wrapper_range(14..23),
                        if_blocks: vec![{
                            let mut b = BTreeMap::new();
                            b.insert(
                                BigUint::from(1u64),
                                vec![Unit::TokenWithRange(&t_deep_nested)]
                                    .into_wrapper_range(14..23),
                            );
                            IfBlock {
                                branches: b,
                                end_if: ().into_wrapper_range(14..23),
                            }
                        }],
                        end_random: ().into_wrapper_range(14..23),
                    }],
                }],
                end_sw: ().into_wrapper_range(14..23),
            }]
            .into_wrapper_range(14..23),
        );

        let units = vec![Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u64),
            }
            .into_wrapper_range(14..23),
            if_blocks: vec![IfBlock {
                branches: if_branches,
                end_if: ().into_wrapper_range(14..23),
            }],
            end_random: ().into_wrapper_range(14..23),
        }];
        let mut iter = units.into_iter().peekable();
        let (tokens, _warnings) = parse_control_flow_ast(&mut iter, &mut rng);
        let titles: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t.content() {
                Token::Header { name, args } if name == "TITLE" => Some(args),
                _ => None,
            })
            .collect();
        assert!(titles.iter().any(|s| **s == "DEEP_NESTED"));
    }
}
