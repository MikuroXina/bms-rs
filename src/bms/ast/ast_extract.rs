use num::BigUint;

use crate::bms::{
    ast::ast_build::{BlockValue, CaseBranch, CaseBranchValue, IfBlock, Unit},
    lex::{TokenStream, token::TokenWithPos},
};

use super::AstRoot;

/// Extracts tokens from the AST and returns them as a TokenStream.
/// This function flattens the AST structure by resolving all control flow constructs
/// and returning the actual tokens that would be executed.
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

/// Extracts tokens from a Random block.
/// For Random blocks, we need to select one of the If branches based on the block value.
fn extract_random_block<'a>(
    value: BlockValue,
    if_blocks: Vec<IfBlock<'a>>,
    tokens: &mut Vec<TokenWithPos<'a>>,
) {
    let selected_value = match value {
        BlockValue::Random { max: _ } => {
            // For Random blocks, we need to select a value between 1 and max
            // Since we don't have access to RNG here, we'll use the first available branch
            // or default to 1 if no branches are available
            if if_blocks.is_empty() {
                BigUint::from(1u32)
            } else {
                let first_block = &if_blocks[0];
                if first_block.branches.is_empty() {
                    BigUint::from(1u32)
                } else {
                    first_block.branches.keys().next().unwrap().clone()
                }
            }
        }
        BlockValue::Set { value } => value,
    };

    // Find the matching If branch
    for if_block in if_blocks {
        if let Some(branch) = if_block.branches.get(&selected_value) {
            extract_units(branch.tokens.clone(), tokens);
            break;
        }
    }
}

/// Extracts tokens from a Switch block.
/// For Switch blocks, we need to select one of the Case branches based on the block value.
fn extract_switch_block<'a>(
    value: BlockValue,
    cases: Vec<CaseBranch<'a>>,
    tokens: &mut Vec<TokenWithPos<'a>>,
) {
    let selected_value = match value {
        BlockValue::Random { max: _ } => {
            // For Switch blocks with Random, we need to select a value between 1 and max
            // Since we don't have access to RNG here, we'll use the first available case
            // or default to 1 if no cases are available
            if cases.is_empty() {
                BigUint::from(1u32)
            } else {
                match &cases[0].value {
                    CaseBranchValue::Case(value) => value.clone(),
                    CaseBranchValue::Def => BigUint::from(1u32),
                }
            }
        }
        BlockValue::Set { value } => value,
    };

    // Find the matching Case branch
    for case in cases {
        match &case.value {
            CaseBranchValue::Case(case_value) => {
                if case_value == &selected_value {
                    extract_units(case.tokens, tokens);
                    return;
                }
            }
            CaseBranchValue::Def => {
                // Store Def branch for fallback
                extract_units(case.tokens, tokens);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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

        let units = tokens
            .iter()
            .map(Unit::TokenWithPos)
            .collect::<Vec<_>>();

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
            tokens: if_tokens
                .iter()
                .map(Unit::TokenWithPos)
                .collect::<Vec<_>>(),
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

        assert_eq!(extracted.tokens.len(), 2);
        assert!(matches!(extracted.tokens[0].content(), Title("00550000")));
        assert!(matches!(extracted.tokens[1].content(), Title("00006600")));
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

        assert_eq!(extracted.tokens.len(), 2);
        assert!(matches!(extracted.tokens[0].content(), Title("11111111")));
        assert!(matches!(extracted.tokens[1].content(), Title("22222222")));
    }
}
