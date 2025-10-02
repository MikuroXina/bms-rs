use crate::bms::{
    ast::structure::{BlockValue, CaseBranch, CaseBranchValue, IfBlock, Unit},
    command::mixin::{SourceRangeMixin, SourceRangeMixinExt},
    lex::token::{ControlFlow, Token, TokenWithRange},
};

/// Recursively extracts tokens from AST units.
pub(super) fn extract_units<'a>(
    units: impl IntoIterator<Item = Unit<'a>>,
) -> Vec<TokenWithRange<'a>> {
    let mut tokens = Vec::new();
    for unit in units {
        match unit {
            Unit::TokenWithRange(token) => {
                tokens.push(token.clone());
            }
            Unit::RandomBlock {
                value,
                if_blocks,
                end_random,
            } => {
                tokens.extend(extract_random_block(&value, if_blocks, &end_random));
            }
            Unit::SwitchBlock {
                value,
                cases,
                end_sw,
            } => {
                tokens.extend(extract_switch_block(&value, cases, &end_sw));
            }
        }
    }
    tokens
}

/// Extracts all tokens from a Random block.
/// This function outputs ALL branches in the Random block, not just the selected one.
fn extract_random_block<'a>(
    value: &SourceRangeMixin<BlockValue>,
    if_blocks: impl IntoIterator<Item = IfBlock<'a>>,
    end_random: &SourceRangeMixin<()>,
) -> Vec<TokenWithRange<'a>> {
    let mut tokens: Vec<TokenWithRange<'a>> = Vec::new();

    let random_value = match value.content().clone() {
        BlockValue::Random { max } => max,
        BlockValue::Set { value } => value,
    };

    // Add the Random token at the original header position
    tokens.push(Token::ControlFlow(ControlFlow::Random(random_value)).into_wrapper(value));

    // Extract all If blocks and their branches
    for IfBlock { branches, end_if } in if_blocks {
        for (branch_key, units) in branches {
            // Add the If token using the branch wrapper position
            let if_token = Token::ControlFlow(ControlFlow::If(branch_key)).into_wrapper(&units);
            tokens.push(if_token);

            // Extract all tokens in this branch
            let units_vec = units.content().clone();
            tokens.extend(extract_units(units_vec));

            // Add the EndIf token at recorded position
            tokens.push(Token::ControlFlow(ControlFlow::EndIf).into_wrapper(&end_if));
        }
    }

    // Add the EndRandom token at recorded position
    tokens.push(Token::ControlFlow(ControlFlow::EndRandom).into_wrapper(end_random));

    tokens
}

/// Extracts all tokens from a Switch block.
/// This function outputs ALL branches in the Switch block, not just the selected one.
fn extract_switch_block<'a>(
    value: &SourceRangeMixin<BlockValue>,
    cases: impl IntoIterator<Item = CaseBranch<'a>>,
    end_sw: &SourceRangeMixin<()>,
) -> Vec<TokenWithRange<'a>> {
    let mut tokens = Vec::new();

    // Add the Switch token
    let switch_value = match value.content().clone() {
        BlockValue::Random { max } => max,
        BlockValue::Set { value } => value,
    };

    tokens.push(Token::ControlFlow(ControlFlow::Switch(switch_value)).into_wrapper(value));

    // Extract all case branches
    for CaseBranch { value, units } in cases {
        match value.content().clone() {
            CaseBranchValue::Case(case_value) => {
                // Add the Case token
                tokens.push(
                    Token::ControlFlow(ControlFlow::Case(case_value.clone())).into_wrapper(&value),
                );

                // Extract all tokens in this case
                tokens.extend(extract_units(units));

                // Add the Skip token
                let (_skip_start, skip_end) =
                    tokens.last().map_or(value.as_span(), |t| t.as_span());
                let skip_token =
                    Token::ControlFlow(ControlFlow::Skip).into_wrapper_range(skip_end..skip_end);
                tokens.push(skip_token);
            }
            CaseBranchValue::Def => {
                // Add the Def token
                tokens.push(Token::ControlFlow(ControlFlow::Def).into_wrapper(&value));

                // Extract all tokens in this def branch
                tokens.extend(extract_units(units));

                // Add the Skip token
                let (_skip_start, skip_end) =
                    tokens.last().map_or(value.as_span(), |t| t.as_span());
                let skip_token =
                    Token::ControlFlow(ControlFlow::Skip).into_wrapper_range(skip_end..skip_end);
                tokens.push(skip_token);
            }
        }
    }

    // Add the EndSwitch token at recorded ENDSW position
    tokens.push(Token::ControlFlow(ControlFlow::EndSwitch).into_wrapper(end_sw));

    tokens
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use num::BigUint;

    use super::*;
    use crate::bms::{
        ast::{AstRoot, structure::IfBlock},
        command::mixin::SourceRangeMixinExt,
        lex::token::Token,
    };

    #[test]
    fn test_extract_simple_tokens() {
        let tokens = vec![
            Token::header("TITLE", "11000000"),
            Token::header("TITLE", "00220000"),
            Token::header("TITLE", "00000044"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let units = tokens.iter().map(Unit::TokenWithRange).collect::<Vec<_>>();

        let ast_root = AstRoot { units };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::header("TITLE", "11000000"),
                Token::header("TITLE", "00220000"),
                Token::header("TITLE", "00000044"),
            ]
        );
    }

    #[test]
    fn test_extract_random_block() {
        use crate::bms::lex::token::ControlFlow as CF;
        let if_tokens = vec![
            Token::header("TITLE", "00550000"),
            Token::header("TITLE", "00006600"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let if_branch = if_tokens
            .iter()
            .map(Unit::TokenWithRange)
            .collect::<Vec<_>>();

        let mut branches = BTreeMap::new();
        branches.insert(BigUint::from(1u32), if_branch.into_wrapper_range(14..23));

        let if_block = IfBlock {
            branches,
            end_if: ().into_wrapper_range(0..0),
        };
        let random_block = Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_range(14..23),
            if_blocks: vec![if_block],
            end_random: ().into_wrapper_range(15..23),
        };

        let ast_root = AstRoot {
            units: vec![random_block],
        };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::ControlFlow(CF::Random(BigUint::from(1u32))),
                Token::ControlFlow(CF::If(BigUint::from(1u32))),
                Token::header("TITLE", "00550000"),
                Token::header("TITLE", "00006600"),
                Token::ControlFlow(CF::EndIf),
                Token::ControlFlow(CF::EndRandom),
            ]
        );
    }

    #[test]
    fn test_extract_switch_block() {
        use crate::bms::lex::token::ControlFlow as CF;
        let case_tokens = vec![
            Token::header("TITLE", "11111111"),
            Token::header("TITLE", "22222222"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let case_branch = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_range(14..23),
            units: case_tokens
                .iter()
                .map(Unit::TokenWithRange)
                .collect::<Vec<_>>(),
        };

        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_range(14..23),
            cases: vec![case_branch],
            end_sw: ().into_wrapper_range(14..23),
        };
        let ast_root = AstRoot {
            units: vec![switch_block],
        };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::ControlFlow(CF::Switch(BigUint::from(1u32))),
                Token::ControlFlow(CF::Case(BigUint::from(1u32))),
                Token::header("TITLE", "11111111"),
                Token::header("TITLE", "22222222"),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::EndSwitch),
            ]
        );
    }

    #[test]
    fn test_extract_switch_block_with_def() {
        use crate::bms::lex::token::ControlFlow as CF;
        let def_tokens = vec![
            Token::header("TITLE", "33333333"),
            Token::header("TITLE", "44444444"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let def_branch = CaseBranch {
            value: CaseBranchValue::Def.into_wrapper_range(14..23),
            units: def_tokens
                .iter()
                .map(Unit::TokenWithRange)
                .collect::<Vec<_>>(),
        };

        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(2u32), // Different from any case
            }
            .into_wrapper_range(14..23),
            cases: vec![def_branch],
            end_sw: ().into_wrapper_range(14..23),
        };
        let ast_root = AstRoot {
            units: vec![switch_block],
        };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::ControlFlow(CF::Switch(BigUint::from(2u32))),
                Token::ControlFlow(CF::Def),
                Token::header("TITLE", "33333333"),
                Token::header("TITLE", "44444444"),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::EndSwitch),
            ]
        );
    }

    #[test]
    fn test_extract_empty_random_block() {
        let random_block = Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_range(14..23),
            if_blocks: vec![],
            end_random: ().into_wrapper_range(15..23),
        };

        let ast_root = AstRoot {
            units: vec![random_block],
        };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::ControlFlow(ControlFlow::Random(BigUint::from(1u32))),
                Token::ControlFlow(ControlFlow::EndRandom),
            ]
        );
    }

    #[test]
    fn test_extract_empty_switch_block() {
        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_range(14..23),
            cases: vec![],
            end_sw: ().into_wrapper_range(14..23),
        };
        let ast_root = AstRoot {
            units: vec![switch_block],
        };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::ControlFlow(ControlFlow::Switch(BigUint::from(1u32))),
                Token::ControlFlow(ControlFlow::EndSwitch),
            ]
        );
    }

    #[test]
    fn test_extract_multiple_random_branches() {
        use crate::bms::lex::token::ControlFlow as CF;

        // Create two different If branches
        let if_tokens_1 = vec![
            Token::header("TITLE", "Branch1_Token1"),
            Token::header("TITLE", "Branch1_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let if_tokens_2 = vec![
            Token::header("TITLE", "Branch2_Token1"),
            Token::header("TITLE", "Branch2_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let if_branch_1 = if_tokens_1
            .iter()
            .map(Unit::TokenWithRange)
            .collect::<Vec<_>>();

        let if_branch_2 = if_tokens_2
            .iter()
            .map(Unit::TokenWithRange)
            .collect::<Vec<_>>();

        let mut branches = BTreeMap::new();
        branches.insert(BigUint::from(1u32), if_branch_1.into_wrapper_range(14..23));
        branches.insert(BigUint::from(2u32), if_branch_2.into_wrapper_range(14..23));

        let if_block = IfBlock {
            branches,
            end_if: ().into_wrapper_range(0..0),
        };
        let random_block = Unit::RandomBlock {
            value: BlockValue::Random {
                max: BigUint::from(2u32),
            }
            .into_wrapper_range(14..23),
            if_blocks: vec![if_block],
            end_random: ().into_wrapper_range(15..23),
        };

        let ast_root = AstRoot {
            units: vec![random_block],
        };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::ControlFlow(CF::Random(BigUint::from(2u32))),
                Token::ControlFlow(CF::If(BigUint::from(1u32))),
                Token::header("TITLE", "Branch1_Token1"),
                Token::header("TITLE", "Branch1_Token2"),
                Token::ControlFlow(CF::EndIf),
                Token::ControlFlow(CF::If(BigUint::from(2u32))),
                Token::header("TITLE", "Branch2_Token1"),
                Token::header("TITLE", "Branch2_Token2"),
                Token::ControlFlow(CF::EndIf),
                Token::ControlFlow(CF::EndRandom),
            ]
        );
    }

    #[test]
    fn test_extract_multiple_switch_cases() {
        use crate::bms::lex::token::ControlFlow as CF;

        // Create Case branch 1
        let case_tokens_1 = vec![
            Token::header("TITLE", "Case1_Token1"),
            Token::header("TITLE", "Case1_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        // Create Case branch 2
        let case_tokens_2 = vec![
            Token::header("TITLE", "Case2_Token1"),
            Token::header("TITLE", "Case2_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        // Create Def branch
        let def_tokens = vec![
            Token::header("TITLE", "Def_Token1"),
            Token::header("TITLE", "Def_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let case_branch_1 = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_range(14..23),
            units: case_tokens_1
                .iter()
                .map(Unit::TokenWithRange)
                .collect::<Vec<_>>(),
        };

        let case_branch_2 = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(2u32)).into_wrapper_range(14..23),
            units: case_tokens_2
                .iter()
                .map(Unit::TokenWithRange)
                .collect::<Vec<_>>(),
        };

        let def_branch = CaseBranch {
            value: CaseBranchValue::Def.into_wrapper_range(14..23),
            units: def_tokens
                .iter()
                .map(Unit::TokenWithRange)
                .collect::<Vec<_>>(),
        };

        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Random {
                max: BigUint::from(3u32),
            }
            .into_wrapper_range(14..23),
            cases: vec![case_branch_1, case_branch_2, def_branch],
            end_sw: ().into_wrapper_range(14..23),
        };

        let ast_root = AstRoot {
            units: vec![switch_block],
        };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::ControlFlow(CF::Switch(BigUint::from(3u32))),
                Token::ControlFlow(CF::Case(BigUint::from(1u32))),
                Token::header("TITLE", "Case1_Token1"),
                Token::header("TITLE", "Case1_Token2"),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::Case(BigUint::from(2u32))),
                Token::header("TITLE", "Case2_Token1"),
                Token::header("TITLE", "Case2_Token2"),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::Def),
                Token::header("TITLE", "Def_Token1"),
                Token::header("TITLE", "Def_Token2"),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::EndSwitch),
            ]
        );
    }

    #[test]
    fn test_extract_def_first_in_switch() {
        use crate::bms::lex::token::ControlFlow as CF;

        // Create Def branch first, then Case branches
        let def_tokens = vec![
            Token::header("TITLE", "DefFirst_Token1"),
            Token::header("TITLE", "DefFirst_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let case_tokens_1 = vec![
            Token::header("TITLE", "Case1_Token1"),
            Token::header("TITLE", "Case1_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let case_tokens_2 = vec![
            Token::header("TITLE", "Case2_Token1"),
            Token::header("TITLE", "Case2_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let def_branch = CaseBranch {
            value: CaseBranchValue::Def.into_wrapper_range(14..23),
            units: def_tokens
                .iter()
                .map(Unit::TokenWithRange)
                .collect::<Vec<_>>(),
        };

        let case_branch_1 = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_range(14..23),
            units: case_tokens_1
                .iter()
                .map(Unit::TokenWithRange)
                .collect::<Vec<_>>(),
        };

        let case_branch_2 = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(2u32)).into_wrapper_range(14..23),
            units: case_tokens_2
                .iter()
                .map(Unit::TokenWithRange)
                .collect::<Vec<_>>(),
        };

        // Def first, then Case branches
        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Random {
                max: BigUint::from(2u32),
            }
            .into_wrapper_range(14..23),
            cases: vec![def_branch, case_branch_1, case_branch_2],
            end_sw: ().into_wrapper_range(14..23),
        };

        let ast_root = AstRoot {
            units: vec![switch_block],
        };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::ControlFlow(CF::Switch(BigUint::from(2u32))),
                Token::ControlFlow(CF::Def),
                Token::header("TITLE", "DefFirst_Token1"),
                Token::header("TITLE", "DefFirst_Token2"),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::Case(BigUint::from(1u32))),
                Token::header("TITLE", "Case1_Token1"),
                Token::header("TITLE", "Case1_Token2"),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::Case(BigUint::from(2u32))),
                Token::header("TITLE", "Case2_Token1"),
                Token::header("TITLE", "Case2_Token2"),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::EndSwitch),
            ]
        );
    }

    #[test]
    fn test_extract_nested_random_in_switch() {
        use crate::bms::lex::token::ControlFlow as CF;

        // Create a Switch block with nested Random block
        let nested_random_tokens = vec![
            Token::header("TITLE", "NestedRandom_Token1"),
            Token::header("TITLE", "NestedRandom_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let _case_tokens = vec![
            Token::header("TITLE", "Case_Token1"),
            Token::header("TITLE", "Case_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        // Create nested Random block
        let nested_if_branch = nested_random_tokens
            .iter()
            .map(Unit::TokenWithRange)
            .collect::<Vec<_>>();

        let mut nested_branches = BTreeMap::new();
        nested_branches.insert(
            BigUint::from(1u32),
            nested_if_branch.into_wrapper_range(14..23),
        );

        let nested_random_block = Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_range(14..23),
            if_blocks: vec![IfBlock {
                branches: nested_branches,
                end_if: ().into_wrapper_range(0..0),
            }],
            end_random: ().into_wrapper_range(15..23),
        };

        let case_branch = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_range(14..23),
            units: vec![nested_random_block],
        };

        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_range(14..23),
            cases: vec![case_branch],
            end_sw: ().into_wrapper_range(14..23),
        };

        let ast_root = AstRoot {
            units: vec![switch_block],
        };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::ControlFlow(CF::Switch(BigUint::from(1u32))),
                Token::ControlFlow(CF::Case(BigUint::from(1u32))),
                Token::ControlFlow(CF::Random(BigUint::from(1u32))),
                Token::ControlFlow(CF::If(BigUint::from(1u32))),
                Token::header("TITLE", "NestedRandom_Token1"),
                Token::header("TITLE", "NestedRandom_Token2"),
                Token::ControlFlow(CF::EndIf),
                Token::ControlFlow(CF::EndRandom),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::EndSwitch),
            ]
        );
    }

    #[test]
    fn test_extract_nested_switch_in_random() {
        use crate::bms::lex::token::ControlFlow as CF;

        // Create a Random block with nested Switch block
        let nested_switch_tokens = vec![
            Token::header("TITLE", "NestedSwitch_Token1"),
            Token::header("TITLE", "NestedSwitch_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let _if_tokens = vec![
            Token::header("TITLE", "If_Token1"),
            Token::header("TITLE", "If_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        // Create nested Switch block
        let nested_case_branch = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_range(14..23),
            units: nested_switch_tokens
                .iter()
                .map(Unit::TokenWithRange)
                .collect::<Vec<_>>(),
        };

        let nested_switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_range(14..23),
            cases: vec![nested_case_branch],
            end_sw: ().into_wrapper_range(14..23),
        };

        let if_branch = vec![nested_switch_block];

        let mut branches = BTreeMap::new();
        branches.insert(BigUint::from(1u32), if_branch.into_wrapper_range(14..23));

        let random_block = Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_range(14..23),
            if_blocks: vec![IfBlock {
                branches,
                end_if: ().into_wrapper_range(0..0),
            }],
            end_random: ().into_wrapper_range(15..23),
        };

        let ast_root = AstRoot {
            units: vec![random_block],
        };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::ControlFlow(CF::Random(BigUint::from(1u32))),
                Token::ControlFlow(CF::If(BigUint::from(1u32))),
                Token::ControlFlow(CF::Switch(BigUint::from(1u32))),
                Token::ControlFlow(CF::Case(BigUint::from(1u32))),
                Token::header("TITLE", "NestedSwitch_Token1"),
                Token::header("TITLE", "NestedSwitch_Token2"),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::EndSwitch),
                Token::ControlFlow(CF::EndIf),
                Token::ControlFlow(CF::EndRandom),
            ]
        );
    }

    #[test]
    fn test_extract_complex_nested_structure() {
        use crate::bms::lex::token::ControlFlow as CF;

        // Create a complex nested structure: Switch -> Case -> Random -> If -> Switch -> Case
        let innermost_tokens = vec![
            Token::header("TITLE", "Innermost_Token1"),
            Token::header("TITLE", "Innermost_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let _middle_tokens = vec![
            Token::header("TITLE", "Middle_Token1"),
            Token::header("TITLE", "Middle_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let _outer_tokens = vec![
            Token::header("TITLE", "Outer_Token1"),
            Token::header("TITLE", "Outer_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        // Innermost Switch block
        let innermost_case = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_range(14..23),
            units: innermost_tokens
                .iter()
                .map(Unit::TokenWithRange)
                .collect::<Vec<_>>(),
        };

        let innermost_switch = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_range(14..23),
            cases: vec![innermost_case],
            end_sw: ().into_wrapper_range(14..23),
        };

        // Middle Random block
        let middle_if_branch = vec![innermost_switch];

        let mut middle_branches = BTreeMap::new();
        middle_branches.insert(
            BigUint::from(1u32),
            middle_if_branch.into_wrapper_range(14..23),
        );

        let middle_random = Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_range(14..23),
            if_blocks: vec![IfBlock {
                branches: middle_branches,
                end_if: ().into_wrapper_range(0..0),
            }],
            end_random: ().into_wrapper_range(15..23),
        };

        // Outer Switch block
        let outer_case = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_range(14..23),
            units: vec![middle_random],
        };

        let outer_switch = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_range(14..23),
            cases: vec![outer_case],
            end_sw: ().into_wrapper_range(14..23),
        };

        let ast_root = AstRoot {
            units: vec![outer_switch],
        };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::ControlFlow(CF::Switch(BigUint::from(1u32))),
                Token::ControlFlow(CF::Case(BigUint::from(1u32))),
                Token::ControlFlow(CF::Random(BigUint::from(1u32))),
                Token::ControlFlow(CF::If(BigUint::from(1u32))),
                Token::ControlFlow(CF::Switch(BigUint::from(1u32))),
                Token::ControlFlow(CF::Case(BigUint::from(1u32))),
                Token::header("TITLE", "Innermost_Token1"),
                Token::header("TITLE", "Innermost_Token2"),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::EndSwitch),
                Token::ControlFlow(CF::EndIf),
                Token::ControlFlow(CF::EndRandom),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::EndSwitch),
            ]
        );
    }

    #[test]
    fn test_extract_multiple_def_branches() {
        use crate::bms::lex::token::ControlFlow as CF;

        // Create a Switch block with multiple Def branches (this should be handled gracefully)
        let def_tokens_1 = vec![
            Token::header("TITLE", "Def1_Token1"),
            Token::header("TITLE", "Def1_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let def_tokens_2 = vec![
            Token::header("TITLE", "Def2_Token1"),
            Token::header("TITLE", "Def2_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let case_tokens = vec![
            Token::header("TITLE", "Case_Token1"),
            Token::header("TITLE", "Case_Token2"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();

        let def_branch_1 = CaseBranch {
            value: CaseBranchValue::Def.into_wrapper_range(14..23),
            units: def_tokens_1
                .iter()
                .map(Unit::TokenWithRange)
                .collect::<Vec<_>>(),
        };

        let def_branch_2 = CaseBranch {
            value: CaseBranchValue::Def.into_wrapper_range(14..23),
            units: def_tokens_2
                .iter()
                .map(Unit::TokenWithRange)
                .collect::<Vec<_>>(),
        };

        let case_branch = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_range(14..23),
            units: case_tokens
                .iter()
                .map(Unit::TokenWithRange)
                .collect::<Vec<_>>(),
        };

        // Multiple Def branches
        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_range(14..23),
            cases: vec![def_branch_1, def_branch_2, case_branch],
            end_sw: ().into_wrapper_range(14..23),
        };

        let ast_root = AstRoot {
            units: vec![switch_block],
        };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::ControlFlow(CF::Switch(BigUint::from(1u32))),
                Token::ControlFlow(CF::Def),
                Token::header("TITLE", "Def1_Token1"),
                Token::header("TITLE", "Def1_Token2"),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::Def),
                Token::header("TITLE", "Def2_Token1"),
                Token::header("TITLE", "Def2_Token2"),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::Case(BigUint::from(1u32))),
                Token::header("TITLE", "Case_Token1"),
                Token::header("TITLE", "Case_Token2"),
                Token::ControlFlow(CF::Skip),
                Token::ControlFlow(CF::EndSwitch),
            ]
        );
    }
}
