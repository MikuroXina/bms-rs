use num::BigUint;

use crate::bms::{
    ast::ast_build::{BlockValue, CaseBranch, CaseBranchValue, IfBlock, Unit},
    command::mixin::SourcePosMixin,
    lex::{
        TokenStream,
        token::{Token, TokenWithPos},
    },
};

use super::AstRoot;

/// Extracts tokens from the AST and returns them as a TokenStream.
/// This function flattens the AST structure by resolving all control flow constructs
/// and returning the actual tokens that would be executed, including control flow tokens.
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
/// Also includes the control flow tokens in the output.
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

    // Create a dummy token for Random command
    let random_token = SourcePosMixin::new(Token::Random(random_value), 0, 0);
    tokens.push(random_token);

    let selected_value = match &value {
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
                    // Use the first available key instead of unwrap
                    if let Some(first_key) = first_block.branches.keys().next() {
                        first_key.clone()
                    } else {
                        BigUint::from(1u32)
                    }
                }
            }
        }
        BlockValue::Set { value } => value.clone(),
    };

    // Find the matching If branch
    for if_block in if_blocks {
        let selected_value_ref = &selected_value;
        if let Some(branch) = if_block.branches.get(selected_value_ref) {
            // Add the If token
            let if_token = SourcePosMixin::new(Token::If(selected_value_ref.clone()), 0, 0);
            tokens.push(if_token);

            extract_units(branch.tokens.clone(), tokens);

            // Add the EndIf token
            let endif_token = SourcePosMixin::new(Token::EndIf, 0, 0);
            tokens.push(endif_token);
            break;
        }
    }

    // Add the EndRandom token
    let endrandom_token = SourcePosMixin::new(Token::EndRandom, 0, 0);
    tokens.push(endrandom_token);
}

/// Extracts tokens from a Switch block.
/// For Switch blocks, we need to select one of the Case branches based on the block value.
/// Also includes the control flow tokens in the output.
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

    let selected_value = match &value {
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
        BlockValue::Set { value } => value.clone(),
    };

    // Find the matching Case branch
    for case in cases {
        match &case.value {
            CaseBranchValue::Case(case_value) => {
                if case_value == &selected_value {
                    // Add the Case token
                    let case_token = SourcePosMixin::new(Token::Case(case_value.clone()), 0, 0);
                    tokens.push(case_token);

                    extract_units(case.tokens, tokens);

                    // Add the Skip token
                    let skip_token = SourcePosMixin::new(Token::Skip, 0, 0);
                    tokens.push(skip_token);
                    break;
                }
            }
            CaseBranchValue::Def => {
                // Store Def branch for fallback
                // Add the Def token
                let def_token = SourcePosMixin::new(Token::Def, 0, 0);
                tokens.push(def_token);

                extract_units(case.tokens, tokens);

                // Add the Skip token
                let skip_token = SourcePosMixin::new(Token::Skip, 0, 0);
                tokens.push(skip_token);
                break;
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
}
