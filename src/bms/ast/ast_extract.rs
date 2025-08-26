use crate::bms::{
    ast::structure::{BlockValue, CaseBranch, CaseBranchValue, IfBlock, Unit},
    command::mixin::SourcePosMixin,
    lex::token::Token,
};

/// Recursively extracts tokens from AST units.
pub(super) fn extract_units<'a>(units: impl IntoIterator<Item = Unit<'a>>) -> Vec<Token<'a>> {
    let mut tokens = Vec::new();
    for unit in units {
        match unit {
            Unit::TokenWithPos(token) => {
                tokens.push(token.content().clone());
            }
            Unit::RandomBlock { value, if_blocks } => {
                tokens.extend(extract_random_block(value, if_blocks));
            }
            Unit::SwitchBlock { value, cases } => {
                tokens.extend(extract_switch_block(value, cases));
            }
        }
    }
    tokens
}

/// Extracts all tokens from a Random block.
/// This function outputs ALL branches in the Random block, not just the selected one.
fn extract_random_block<'a>(
    value: SourcePosMixin<BlockValue>,
    if_blocks: impl IntoIterator<Item = IfBlock<'a>>,
) -> Vec<Token<'a>> {
    let mut tokens = Vec::new();

    // Add the Random token
    let random_value = match value.into_content() {
        BlockValue::Random { max } => max,
        BlockValue::Set { value } => value,
    };

    let random_token = Token::Random(random_value);
    tokens.push(random_token);

    // Extract all If blocks and their branches
    for IfBlock { branches } in if_blocks {
        for (branch_key, units) in branches {
            // Add the If token
            let if_token = Token::If(branch_key);
            tokens.push(if_token);

            // Extract all tokens in this branch
            tokens.extend(extract_units(units.into_content()));

            // Add the EndIf token
            let endif_token = Token::EndIf;
            tokens.push(endif_token);
        }
    }

    // Add the EndRandom token
    let endrandom_token = Token::EndRandom;
    tokens.push(endrandom_token);

    tokens
}

/// Extracts all tokens from a Switch block.
/// This function outputs ALL branches in the Switch block, not just the selected one.
fn extract_switch_block<'a>(
    value: SourcePosMixin<BlockValue>,
    cases: impl IntoIterator<Item = CaseBranch<'a>>,
) -> Vec<Token<'a>> {
    let mut tokens = Vec::new();

    // Add the Switch token
    let switch_value = match value.into_content() {
        BlockValue::Random { max } => max,
        BlockValue::Set { value } => value,
    };

    let switch_token = Token::Switch(switch_value);
    tokens.push(switch_token);

    // Extract all case branches
    for case in cases {
        match case.value.content() {
            CaseBranchValue::Case(case_value) => {
                // Add the Case token
                let case_token = Token::Case(case_value.clone());
                tokens.push(case_token);

                // Extract all tokens in this case
                tokens.extend(extract_units(case.units));

                // Add the Skip token
                let skip_token = Token::Skip;
                tokens.push(skip_token);
            }
            CaseBranchValue::Def => {
                // Add the Def token
                let def_token = Token::Def;
                tokens.push(def_token);

                // Extract all tokens in this def branch
                tokens.extend(extract_units(case.units));

                // Add the Skip token
                let skip_token = Token::Skip;
                tokens.push(skip_token);
            }
        }
    }

    // Add the EndSwitch token
    let endswitch_token = Token::EndSwitch;
    tokens.push(endswitch_token);

    tokens
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use num::BigUint;

    use super::*;
    use crate::bms::{
        ast::{AstRoot, structure::IfBlock},
        command::mixin::SourcePosMixinExt,
        lex::token::Token,
    };

    #[test]
    fn test_extract_simple_tokens() {
        use Token::*;
        let tokens = vec![Title("11000000"), Title("00220000"), Title("00000044")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let units = tokens.iter().map(Unit::TokenWithPos).collect::<Vec<_>>();

        let ast_root = AstRoot { units };
        let extracted = ast_root.extract();

        assert_eq!(
            extracted
                .tokens
                .iter()
                .map(|t| t.content())
                .cloned()
                .collect::<Vec<_>>(),
            vec![Title("11000000"), Title("00220000"), Title("00000044"),]
        );
    }

    #[test]
    fn test_extract_random_block() {
        use Token::*;
        let if_tokens = vec![Title("00550000"), Title("00006600")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let if_branch = if_tokens.iter().map(Unit::TokenWithPos).collect::<Vec<_>>();

        let mut branches = BTreeMap::new();
        branches.insert(BigUint::from(1u32), if_branch.into_wrapper_manual(14, 23));

        let if_block = IfBlock { branches };
        let random_block = Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_manual(14, 23),
            if_blocks: vec![if_block],
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
                Random(BigUint::from(1u32)),
                If(BigUint::from(1u32)),
                Title("00550000"),
                Title("00006600"),
                EndIf,
                EndRandom,
            ]
        );
    }

    #[test]
    fn test_extract_switch_block() {
        use Token::*;
        let case_tokens = vec![Title("11111111"), Title("22222222")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let case_branch = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_manual(14, 23),
            units: case_tokens
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_manual(14, 23),
            cases: vec![case_branch],
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
                Switch(BigUint::from(1u32)),
                Case(BigUint::from(1u32)),
                Title("11111111"),
                Title("22222222"),
                Skip,
                EndSwitch,
            ]
        );
    }

    #[test]
    fn test_extract_switch_block_with_def() {
        use Token::*;
        let def_tokens = vec![Title("33333333"), Title("44444444")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let def_branch = CaseBranch {
            value: CaseBranchValue::Def.into_wrapper_manual(14, 23),
            units: def_tokens
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(2u32), // Different from any case
            }
            .into_wrapper_manual(14, 23),
            cases: vec![def_branch],
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
                Switch(BigUint::from(2u32)),
                Def,
                Title("33333333"),
                Title("44444444"),
                Skip,
                EndSwitch,
            ]
        );
    }

    #[test]
    fn test_extract_empty_random_block() {
        let random_block = Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_manual(14, 23),
            if_blocks: vec![],
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
            vec![Token::Random(BigUint::from(1u32)), Token::EndRandom,]
        );
    }

    #[test]
    fn test_extract_empty_switch_block() {
        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_manual(14, 23),
            cases: vec![],
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
            vec![Token::Switch(BigUint::from(1u32)), Token::EndSwitch,]
        );
    }

    #[test]
    fn test_extract_multiple_random_branches() {
        use Token::*;

        // Create two different If branches
        let if_tokens_1 = vec![Title("Branch1_Token1"), Title("Branch1_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let if_tokens_2 = vec![Title("Branch2_Token1"), Title("Branch2_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let if_branch_1 = if_tokens_1
            .iter()
            .map(Unit::TokenWithPos)
            .collect::<Vec<_>>();

        let if_branch_2 = if_tokens_2
            .iter()
            .map(Unit::TokenWithPos)
            .collect::<Vec<_>>();

        let mut branches = BTreeMap::new();
        branches.insert(BigUint::from(1u32), if_branch_1.into_wrapper_manual(14, 23));
        branches.insert(BigUint::from(2u32), if_branch_2.into_wrapper_manual(14, 23));

        let if_block = IfBlock { branches };
        let random_block = Unit::RandomBlock {
            value: BlockValue::Random {
                max: BigUint::from(2u32),
            }
            .into_wrapper_manual(14, 23),
            if_blocks: vec![if_block],
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
                Random(BigUint::from(2u32)),
                If(BigUint::from(1u32)),
                Title("Branch1_Token1"),
                Title("Branch1_Token2"),
                EndIf,
                If(BigUint::from(2u32)),
                Title("Branch2_Token1"),
                Title("Branch2_Token2"),
                EndIf,
                EndRandom,
            ]
        );
    }

    #[test]
    fn test_extract_multiple_switch_cases() {
        use Token::*;

        // Create Case branch 1
        let case_tokens_1 = vec![Title("Case1_Token1"), Title("Case1_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        // Create Case branch 2
        let case_tokens_2 = vec![Title("Case2_Token1"), Title("Case2_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        // Create Def branch
        let def_tokens = vec![Title("Def_Token1"), Title("Def_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let case_branch_1 = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_manual(14, 23),
            units: case_tokens_1
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let case_branch_2 = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(2u32)).into_wrapper_manual(14, 23),
            units: case_tokens_2
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let def_branch = CaseBranch {
            value: CaseBranchValue::Def.into_wrapper_manual(14, 23),
            units: def_tokens
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Random {
                max: BigUint::from(3u32),
            }
            .into_wrapper_manual(14, 23),
            cases: vec![case_branch_1, case_branch_2, def_branch],
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
                Switch(BigUint::from(3u32)),
                Case(BigUint::from(1u32)),
                Title("Case1_Token1"),
                Title("Case1_Token2"),
                Skip,
                Case(BigUint::from(2u32)),
                Title("Case2_Token1"),
                Title("Case2_Token2"),
                Skip,
                Def,
                Title("Def_Token1"),
                Title("Def_Token2"),
                Skip,
                EndSwitch,
            ]
        );
    }

    #[test]
    fn test_extract_def_first_in_switch() {
        use Token::*;

        // Create Def branch first, then Case branches
        let def_tokens = vec![Title("DefFirst_Token1"), Title("DefFirst_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let case_tokens_1 = vec![Title("Case1_Token1"), Title("Case1_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let case_tokens_2 = vec![Title("Case2_Token1"), Title("Case2_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let def_branch = CaseBranch {
            value: CaseBranchValue::Def.into_wrapper_manual(14, 23),
            units: def_tokens
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let case_branch_1 = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_manual(14, 23),
            units: case_tokens_1
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let case_branch_2 = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(2u32)).into_wrapper_manual(14, 23),
            units: case_tokens_2
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        // Def first, then Case branches
        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Random {
                max: BigUint::from(2u32),
            }
            .into_wrapper_manual(14, 23),
            cases: vec![def_branch, case_branch_1, case_branch_2],
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
                Switch(BigUint::from(2u32)),
                Def,
                Title("DefFirst_Token1"),
                Title("DefFirst_Token2"),
                Skip,
                Case(BigUint::from(1u32)),
                Title("Case1_Token1"),
                Title("Case1_Token2"),
                Skip,
                Case(BigUint::from(2u32)),
                Title("Case2_Token1"),
                Title("Case2_Token2"),
                Skip,
                EndSwitch,
            ]
        );
    }

    #[test]
    fn test_extract_nested_random_in_switch() {
        use Token::*;

        // Create a Switch block with nested Random block
        let nested_random_tokens = vec![Title("NestedRandom_Token1"), Title("NestedRandom_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let _case_tokens = vec![Title("Case_Token1"), Title("Case_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        // Create nested Random block
        let nested_if_branch = nested_random_tokens
            .iter()
            .map(Unit::TokenWithPos)
            .collect::<Vec<_>>();

        let mut nested_branches = BTreeMap::new();
        nested_branches.insert(
            BigUint::from(1u32),
            nested_if_branch.into_wrapper_manual(14, 23),
        );

        let nested_random_block = Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_manual(14, 23),
            if_blocks: vec![IfBlock {
                branches: nested_branches,
            }],
        };

        let case_branch = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_manual(14, 23),
            units: vec![nested_random_block],
        };

        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_manual(14, 23),
            cases: vec![case_branch],
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
                Switch(BigUint::from(1u32)),
                Case(BigUint::from(1u32)),
                Random(BigUint::from(1u32)),
                If(BigUint::from(1u32)),
                Title("NestedRandom_Token1"),
                Title("NestedRandom_Token2"),
                EndIf,
                EndRandom,
                Skip,
                EndSwitch,
            ]
        );
    }

    #[test]
    fn test_extract_nested_switch_in_random() {
        use Token::*;

        // Create a Random block with nested Switch block
        let nested_switch_tokens = vec![Title("NestedSwitch_Token1"), Title("NestedSwitch_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let _if_tokens = vec![Title("If_Token1"), Title("If_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        // Create nested Switch block
        let nested_case_branch = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_manual(14, 23),
            units: nested_switch_tokens
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let nested_switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_manual(14, 23),
            cases: vec![nested_case_branch],
        };

        let if_branch = vec![nested_switch_block];

        let mut branches = BTreeMap::new();
        branches.insert(BigUint::from(1u32), if_branch.into_wrapper_manual(14, 23));

        let random_block = Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_manual(14, 23),
            if_blocks: vec![IfBlock { branches }],
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
                Random(BigUint::from(1u32)),
                If(BigUint::from(1u32)),
                Switch(BigUint::from(1u32)),
                Case(BigUint::from(1u32)),
                Title("NestedSwitch_Token1"),
                Title("NestedSwitch_Token2"),
                Skip,
                EndSwitch,
                EndIf,
                EndRandom,
            ]
        );
    }

    #[test]
    fn test_extract_complex_nested_structure() {
        use Token::*;

        // Create a complex nested structure: Switch -> Case -> Random -> If -> Switch -> Case
        let innermost_tokens = vec![Title("Innermost_Token1"), Title("Innermost_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let _middle_tokens = vec![Title("Middle_Token1"), Title("Middle_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let _outer_tokens = vec![Title("Outer_Token1"), Title("Outer_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        // Innermost Switch block
        let innermost_case = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_manual(14, 23),
            units: innermost_tokens
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let innermost_switch = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_manual(14, 23),
            cases: vec![innermost_case],
        };

        // Middle Random block
        let middle_if_branch = vec![innermost_switch];

        let mut middle_branches = BTreeMap::new();
        middle_branches.insert(
            BigUint::from(1u32),
            middle_if_branch.into_wrapper_manual(14, 23),
        );

        let middle_random = Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_manual(14, 23),
            if_blocks: vec![IfBlock {
                branches: middle_branches,
            }],
        };

        // Outer Switch block
        let outer_case = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_manual(14, 23),
            units: vec![middle_random],
        };

        let outer_switch = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_manual(14, 23),
            cases: vec![outer_case],
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
                Switch(BigUint::from(1u32)),
                Case(BigUint::from(1u32)),
                Random(BigUint::from(1u32)),
                If(BigUint::from(1u32)),
                Switch(BigUint::from(1u32)),
                Case(BigUint::from(1u32)),
                Title("Innermost_Token1"),
                Title("Innermost_Token2"),
                Skip,
                EndSwitch,
                EndIf,
                EndRandom,
                Skip,
                EndSwitch,
            ]
        );
    }

    #[test]
    fn test_extract_multiple_def_branches() {
        use Token::*;

        // Create a Switch block with multiple Def branches (this should be handled gracefully)
        let def_tokens_1 = vec![Title("Def1_Token1"), Title("Def1_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let def_tokens_2 = vec![Title("Def2_Token1"), Title("Def2_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let case_tokens = vec![Title("Case_Token1"), Title("Case_Token2")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let def_branch_1 = CaseBranch {
            value: CaseBranchValue::Def.into_wrapper_manual(14, 23),
            units: def_tokens_1
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let def_branch_2 = CaseBranch {
            value: CaseBranchValue::Def.into_wrapper_manual(14, 23),
            units: def_tokens_2
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let case_branch = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(1u32)).into_wrapper_manual(14, 23),
            units: case_tokens
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        // Multiple Def branches
        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            }
            .into_wrapper_manual(14, 23),
            cases: vec![def_branch_1, def_branch_2, case_branch],
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
                Switch(BigUint::from(1u32)),
                Def,
                Title("Def1_Token1"),
                Title("Def1_Token2"),
                Skip,
                Def,
                Title("Def2_Token1"),
                Title("Def2_Token2"),
                Skip,
                Case(BigUint::from(1u32)),
                Title("Case_Token1"),
                Title("Case_Token2"),
                Skip,
                EndSwitch,
            ]
        );
    }
}
