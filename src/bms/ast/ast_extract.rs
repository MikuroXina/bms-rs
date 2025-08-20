use crate::bms::{
    ast::ast_build::{BlockValue, CaseBranch, CaseBranchValue, IfBlock, Unit},
    command::mixin::SourcePosMixin,
    lex::{
        TokenStream,
        token::{Token, TokenWithPos},
    },
};

use super::AstRoot;

/// Extracts all tokens from the AST and returns them as a TokenStream.
/// This function flattens the AST structure and returns ALL tokens contained in the AST,
/// including all branches in Random and Switch blocks. This serves as the inverse of
/// AstRoot::from_token_stream.
pub fn extract_ast_root<'a>(ast_root: AstRoot<'a>) -> TokenStream<'a> {
    let mut tokens = Vec::new();
    extract_units(ast_root.units, &mut tokens);
    TokenStream { tokens }
}

/// Recursively extracts tokens from AST units.
fn extract_units<'a>(units: Vec<Unit<'a>>, tokens: &mut Vec<TokenWithPos<'a>>) {
    for unit in units {
        match unit {
            Unit::TokenWithPos(token) => {
                tokens.push(token.clone());
            }
            Unit::RandomBlock { value, if_blocks } => {
                extract_random_block(value, if_blocks, tokens);
            }
            Unit::SwitchBlock { value, cases } => {
                extract_switch_block(value, cases, tokens);
            }
        }
    }
}

/// Extracts all tokens from a Random block.
/// This function outputs ALL branches in the Random block, not just the selected one.
fn extract_random_block<'a>(
    value: BlockValue,
    if_blocks: Vec<IfBlock<'a>>,
    tokens: &mut Vec<TokenWithPos<'a>>,
) {
    // Add the Random token
    let random_value = match &value {
        BlockValue::Random { max } => max.clone(),
        BlockValue::Set { value } => value.clone(),
    };

    let random_token = SourcePosMixin::new(Token::Random(random_value), 0, 0);
    tokens.push(random_token);

    // Extract all If blocks and their branches
    for if_block in if_blocks {
        // Sort branch keys for consistent output order
        let mut branch_keys: Vec<_> = if_block.branches.keys().collect();
        branch_keys.sort();

        for branch_key in branch_keys {
            if let Some(branch) = if_block.branches.get(branch_key) {
                // Add the If token
                let if_token = SourcePosMixin::new(Token::If(branch_key.clone()), 0, 0);
                tokens.push(if_token);

                // Extract all tokens in this branch
                extract_units(branch.tokens.clone(), tokens);

                // Add the EndIf token
                let endif_token = SourcePosMixin::new(Token::EndIf, 0, 0);
                tokens.push(endif_token);
            }
        }
    }

    // Add the EndRandom token
    let endrandom_token = SourcePosMixin::new(Token::EndRandom, 0, 0);
    tokens.push(endrandom_token);
}

/// Extracts all tokens from a Switch block.
/// This function outputs ALL branches in the Switch block, not just the selected one.
fn extract_switch_block<'a>(
    value: BlockValue,
    cases: Vec<CaseBranch<'a>>,
    tokens: &mut Vec<TokenWithPos<'a>>,
) {
    // Add the Switch token
    let switch_value = match &value {
        BlockValue::Random { max } => max.clone(),
        BlockValue::Set { value } => value.clone(),
    };

    let switch_token = SourcePosMixin::new(Token::Switch(switch_value), 0, 0);
    tokens.push(switch_token);

    // Extract all case branches
    for case in cases {
        match &case.value {
            CaseBranchValue::Case(case_value) => {
                // Add the Case token
                let case_token = SourcePosMixin::new(Token::Case(case_value.clone()), 0, 0);
                tokens.push(case_token);

                // Extract all tokens in this case
                extract_units(case.tokens, tokens);

                // Add the Skip token
                let skip_token = SourcePosMixin::new(Token::Skip, 0, 0);
                tokens.push(skip_token);
            }
            CaseBranchValue::Def => {
                // Add the Def token
                let def_token = SourcePosMixin::new(Token::Def, 0, 0);
                tokens.push(def_token);

                // Extract all tokens in this def branch
                extract_units(case.tokens, tokens);

                // Add the Skip token
                let skip_token = SourcePosMixin::new(Token::Skip, 0, 0);
                tokens.push(skip_token);
            }
        }
    }

    // Add the EndSwitch token
    let endswitch_token = SourcePosMixin::new(Token::EndSwitch, 0, 0);
    tokens.push(endswitch_token);
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use num::BigUint;

    use super::*;
    use crate::bms::{
        ast::ast_build::{IfBlock, IfBranch},
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
        let extracted = extract_ast_root(ast_root);

        assert_eq!(extracted.tokens.len(), 3);
        assert!(matches!(extracted.tokens[0].content(), Title("11000000")));
        assert!(matches!(extracted.tokens[1].content(), Title("00220000")));
        assert!(matches!(extracted.tokens[2].content(), Title("00000044")));
    }

    #[test]
    fn test_extract_random_block() {
        use Token::*;
        let if_tokens = vec![Title("00550000"), Title("00006600")]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();

        let if_branch = IfBranch {
            value: BigUint::from(1u32),
            tokens: if_tokens.iter().map(Unit::TokenWithPos).collect::<Vec<_>>(),
        };

        let mut branches = HashMap::new();
        branches.insert(BigUint::from(1u32), if_branch);

        let if_block = IfBlock { branches };
        let random_block = Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            },
            if_blocks: vec![if_block],
        };

        let ast_root = AstRoot {
            units: vec![random_block],
        };
        let extracted = extract_ast_root(ast_root);

        // Should contain: Random, If, Title1, Title2, EndIf, EndRandom
        assert_eq!(extracted.tokens.len(), 6);
        assert!(matches!(extracted.tokens[0].content(), Random(_)));
        assert!(matches!(extracted.tokens[1].content(), If(_)));
        assert!(matches!(extracted.tokens[2].content(), Title("00550000")));
        assert!(matches!(extracted.tokens[3].content(), Title("00006600")));
        assert!(matches!(extracted.tokens[4].content(), EndIf));
        assert!(matches!(extracted.tokens[5].content(), EndRandom));
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
            value: CaseBranchValue::Case(BigUint::from(1u32)),
            tokens: case_tokens
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            },
            cases: vec![case_branch],
        };
        let ast_root = AstRoot {
            units: vec![switch_block],
        };
        let extracted = extract_ast_root(ast_root);

        // Should contain: Switch, Case, Title1, Title2, Skip, EndSwitch
        assert_eq!(extracted.tokens.len(), 6);
        assert!(matches!(extracted.tokens[0].content(), Switch(_)));
        assert!(matches!(extracted.tokens[1].content(), Case(_)));
        assert!(matches!(extracted.tokens[2].content(), Title("11111111")));
        assert!(matches!(extracted.tokens[3].content(), Title("22222222")));
        assert!(matches!(extracted.tokens[4].content(), Skip));
        assert!(matches!(extracted.tokens[5].content(), EndSwitch));
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
            value: CaseBranchValue::Def,
            tokens: def_tokens
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(2u32), // Different from any case
            },
            cases: vec![def_branch],
        };
        let ast_root = AstRoot {
            units: vec![switch_block],
        };
        let extracted = extract_ast_root(ast_root);

        // Should contain: Switch, Def, Title1, Title2, Skip, EndSwitch
        assert_eq!(extracted.tokens.len(), 6);
        assert!(matches!(extracted.tokens[0].content(), Switch(_)));
        assert!(matches!(extracted.tokens[1].content(), Def));
        assert!(matches!(extracted.tokens[2].content(), Title("33333333")));
        assert!(matches!(extracted.tokens[3].content(), Title("44444444")));
        assert!(matches!(extracted.tokens[4].content(), Skip));
        assert!(matches!(extracted.tokens[5].content(), EndSwitch));
    }

    #[test]
    fn test_extract_empty_random_block() {
        let random_block = Unit::RandomBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            },
            if_blocks: vec![],
        };

        let ast_root = AstRoot {
            units: vec![random_block],
        };
        let extracted = extract_ast_root(ast_root);

        // Should contain: Random, EndRandom (no If/EndIf because no branches)
        assert_eq!(extracted.tokens.len(), 2);
        assert!(matches!(extracted.tokens[0].content(), Token::Random(_)));
        assert!(matches!(extracted.tokens[1].content(), Token::EndRandom));
    }

    #[test]
    fn test_extract_empty_switch_block() {
        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Set {
                value: BigUint::from(1u32),
            },
            cases: vec![],
        };
        let ast_root = AstRoot {
            units: vec![switch_block],
        };
        let extracted = extract_ast_root(ast_root);

        // Should contain: Switch, EndSwitch (no Case/Def because no cases)
        assert_eq!(extracted.tokens.len(), 2);
        assert!(matches!(extracted.tokens[0].content(), Token::Switch(_)));
        assert!(matches!(extracted.tokens[1].content(), Token::EndSwitch));
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

        let if_branch_1 = IfBranch {
            value: BigUint::from(1u32),
            tokens: if_tokens_1
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let if_branch_2 = IfBranch {
            value: BigUint::from(2u32),
            tokens: if_tokens_2
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let mut branches = HashMap::new();
        branches.insert(BigUint::from(1u32), if_branch_1);
        branches.insert(BigUint::from(2u32), if_branch_2);

        let if_block = IfBlock { branches };
        let random_block = Unit::RandomBlock {
            value: BlockValue::Random {
                max: BigUint::from(2u32),
            },
            if_blocks: vec![if_block],
        };

        let ast_root = AstRoot {
            units: vec![random_block],
        };
        let extracted = extract_ast_root(ast_root);

        // Should contain: Random, If(1), Branch1_Token1, Branch1_Token2, EndIf, If(2), Branch2_Token1, Branch2_Token2, EndIf, EndRandom
        assert_eq!(extracted.tokens.len(), 10);
        assert!(matches!(extracted.tokens[0].content(), Random(_)));
        // First branch (value 1)
        assert!(matches!(extracted.tokens[1].content(), If(_)));
        assert!(matches!(
            extracted.tokens[2].content(),
            Title("Branch1_Token1")
        ));
        assert!(matches!(
            extracted.tokens[3].content(),
            Title("Branch1_Token2")
        ));
        assert!(matches!(extracted.tokens[4].content(), EndIf));
        // Second branch (value 2)
        assert!(matches!(extracted.tokens[5].content(), If(_)));
        assert!(matches!(
            extracted.tokens[6].content(),
            Title("Branch2_Token1")
        ));
        assert!(matches!(
            extracted.tokens[7].content(),
            Title("Branch2_Token2")
        ));
        assert!(matches!(extracted.tokens[8].content(), EndIf));
        assert!(matches!(extracted.tokens[9].content(), EndRandom));
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
            value: CaseBranchValue::Case(BigUint::from(1u32)),
            tokens: case_tokens_1
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let case_branch_2 = CaseBranch {
            value: CaseBranchValue::Case(BigUint::from(2u32)),
            tokens: case_tokens_2
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let def_branch = CaseBranch {
            value: CaseBranchValue::Def,
            tokens: def_tokens
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
        };

        let switch_block = Unit::SwitchBlock {
            value: BlockValue::Random {
                max: BigUint::from(3u32),
            },
            cases: vec![case_branch_1, case_branch_2, def_branch],
        };

        let ast_root = AstRoot {
            units: vec![switch_block],
        };
        let extracted = extract_ast_root(ast_root);

        // Should contain: Switch, Case(1), Case1_Token1, Case1_Token2, Skip, Case(2), Case2_Token1, Case2_Token2, Skip, Def, Def_Token1, Def_Token2, Skip, EndSwitch
        assert_eq!(extracted.tokens.len(), 14);
        assert!(matches!(extracted.tokens[0].content(), Switch(_)));
        // Case 1
        assert!(matches!(extracted.tokens[1].content(), Case(_)));
        assert!(matches!(
            extracted.tokens[2].content(),
            Title("Case1_Token1")
        ));
        assert!(matches!(
            extracted.tokens[3].content(),
            Title("Case1_Token2")
        ));
        assert!(matches!(extracted.tokens[4].content(), Skip));
        // Case 2
        assert!(matches!(extracted.tokens[5].content(), Case(_)));
        assert!(matches!(
            extracted.tokens[6].content(),
            Title("Case2_Token1")
        ));
        assert!(matches!(
            extracted.tokens[7].content(),
            Title("Case2_Token2")
        ));
        assert!(matches!(extracted.tokens[8].content(), Skip));
        // Def
        assert!(matches!(extracted.tokens[9].content(), Def));
        assert!(matches!(
            extracted.tokens[10].content(),
            Title("Def_Token1")
        ));
        assert!(matches!(
            extracted.tokens[11].content(),
            Title("Def_Token2")
        ));
        assert!(matches!(extracted.tokens[12].content(), Skip));
        assert!(matches!(extracted.tokens[13].content(), EndSwitch));
    }
}
