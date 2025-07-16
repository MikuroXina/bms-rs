use std::collections::HashMap;

use crate::{
    bms::lex::token::{Token, TokenStream},
    parse::random::ControlFlowRule,
};

/// A unit of AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Unit<'a> {
    /// A token that is not a control flow token.
    Token(&'a Token<'a>),
    /// A Random block. Can contain multiple If blocks.
    RandomBlock {
        value: BlockValue,
        if_blocks: Vec<IfBlock<'a>>,
    },
    /// A Switch block.
    /// Like C++ Programming Language, Switch block can contain multiple Case branches, and a Def branch.
    /// If there is no other Case branch activated, Def branch will be activated.
    /// When executing, the tokens, from the activated branch, to Skip/EndSwitch, will be executed.
    SwitchBlock {
        value: BlockValue,
        cases: Vec<CaseBranch<'a>>,
    },
}

/// The value of a Random/Switch block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum BlockValue {
    /// For Random/Switch, value ranges in [1, max].
    /// IfBranch value must ranges in [1, max].
    Random { max: u64 },
    /// For SetRandom/SetSwitch.
    /// IfBranch value has no limit.
    Set { value: u64 },
}

/// The If block of a Random block. Should contain If/EndIf, can contain ElseIf/Else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct IfBlock<'a> {
    pub branches: HashMap<u64, IfBranch<'a>>,
}

/// The If branch of a If block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct IfBranch<'a> {
    pub value: u64,
    pub tokens: Vec<Unit<'a>>,
}

/// The define of a Case/Def branch in a Switch block.
/// Note: Def can appear in any position. If there is no other Case branch activated, Def will be activated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CaseBranch<'a> {
    pub value: CaseBranchValue,
    pub tokens: Vec<Unit<'a>>,
}

/// The type note of a Case/Def branch.
/// Note: Def can appear in any position. If there is no other Case branch activated, Def will be activated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CaseBranchValue {
    Case(u64),
    Def,
}

pub(super) fn build_control_flow_ast<'a>(
    tokens: &'a TokenStream<'a>,
    error_list: &mut Vec<ControlFlowRule>,
) -> Vec<Unit<'a>> {
    let mut iter = tokens.iter().peekable();
    parse_units(&mut iter, error_list)
}

/// Recursively parse the token stream into a list of AST nodes
fn parse_units<'a, I>(
    iter: &mut std::iter::Peekable<I>,
    error_list: &mut Vec<ControlFlowRule>,
) -> Vec<Unit<'a>>
where
    I: Iterator<Item = &'a Token<'a>>,
{
    let mut result = Vec::new();
    while let Some(token) = iter.peek() {
        match token {
            Token::SetSwitch(_val) | Token::Switch(_val) => {
                result.push(parse_switch_block(iter, error_list));
            }
            Token::Random(_val) | Token::SetRandom(_val) => {
                result.push(parse_random_block(iter, error_list));
            }
            Token::EndIf => {
                error_list.push(ControlFlowRule::UnmatchedEndIf);
                iter.next();
            }
            Token::EndRandom => {
                error_list.push(ControlFlowRule::UnmatchedEndRandom);
                iter.next();
            }
            _ => {
                // Directly collect non-control-flow tokens
                if !token.is_control_flow_token() {
                    result.push(Unit::Token(*token));
                }
                iter.next();
            }
        }
    }
    result
}

/// Parse a Switch/SetSwitch block until EndSwitch
fn parse_switch_block<'a, I>(
    iter: &mut std::iter::Peekable<I>,
    error_list: &mut Vec<ControlFlowRule>,
) -> Unit<'a>
where
    I: Iterator<Item = &'a Token<'a>>,
{
    let token = iter.next().unwrap();
    let block_value = match token {
        Token::SetSwitch(val) => BlockValue::Set { value: *val as u64 },
        Token::Switch(val) => BlockValue::Random { max: *val as u64 },
        _ => unreachable!(),
    };
    let mut cases = Vec::new();
    while let Some(next) = iter.peek() {
        match next {
            Token::Case(case_val) => {
                iter.next();
                let tokens = parse_case_or_def_body(iter, error_list);
                cases.push(CaseBranch {
                    value: CaseBranchValue::Case(*case_val as u64),
                    tokens,
                });
                if let Some(Token::Skip) = iter.peek() {
                    iter.next();
                }
            }
            Token::Def => {
                iter.next();
                let tokens = parse_case_or_def_body(iter, error_list);
                cases.push(CaseBranch {
                    value: CaseBranchValue::Def,
                    tokens,
                });
                if let Some(Token::Skip) = iter.peek() {
                    iter.next();
                }
            }
            Token::EndSwitch => {
                iter.next();
                break;
            }
            Token::EndIf => {
                error_list.push(ControlFlowRule::UnmatchedEndIf);
                iter.next();
            }
            Token::EndRandom => {
                error_list.push(ControlFlowRule::UnmatchedEndRandom);
                iter.next();
            }
            // Automatically complete EndSwitch: break when encountering Random/SetRandom/If/EndRandom/EndIf
            Token::Random(_) | Token::SetRandom(_) | Token::If(_) => {
                break;
            }
            _ => {
                iter.next();
            }
        }
    }
    // If the iterator has ended, also break (i.e., automatically complete EndSwitch)
    Unit::SwitchBlock {
        value: block_value,
        cases,
    }
}

/// Parse the body of a Case/Def branch until a control-flow boundary is encountered
fn parse_case_or_def_body<'a, I>(
    iter: &mut std::iter::Peekable<I>,
    error_list: &mut Vec<ControlFlowRule>,
) -> Vec<Unit<'a>>
where
    I: Iterator<Item = &'a Token<'a>>,
{
    let mut result = Vec::new();
    while let Some(token) = iter.peek() {
        match token {
            Token::Skip | Token::EndSwitch | Token::Case(_) | Token::Def => break,
            Token::SetSwitch(_) | Token::Switch(_) => {
                result.push(parse_switch_block(iter, error_list));
            }
            Token::Random(_) | Token::SetRandom(_) => {
                result.push(parse_random_block(iter, error_list));
            }
            Token::EndIf => {
                error_list.push(ControlFlowRule::UnmatchedEndIf);
                iter.next();
            }
            Token::EndRandom => {
                error_list.push(ControlFlowRule::UnmatchedEndRandom);
                iter.next();
            }
            _ => {
                // Directly collect non-control-flow tokens
                if !token.is_control_flow_token() {
                    result.push(Unit::Token(*token));
                }
                iter.next();
            }
        }
    }
    result
}

/// Parse a Random/SetRandom block until EndRandom
fn parse_random_block<'a, I>(
    iter: &mut std::iter::Peekable<I>,
    error_list: &mut Vec<ControlFlowRule>,
) -> Unit<'a>
where
    I: Iterator<Item = &'a Token<'a>>,
{
    let token = iter.next().unwrap();
    let block_value = match token {
        Token::Random(val) => BlockValue::Random { max: *val as u64 },
        Token::SetRandom(val) => BlockValue::Set { value: *val as u64 },
        _ => unreachable!(),
    };
    let mut if_blocks = Vec::new();
    while let Some(next) = iter.peek() {
        match next {
            Token::If(if_val) => {
                iter.next();
                let (tokens, _has_endif) = parse_if_block_body(iter, error_list);
                let mut branches = HashMap::new();
                branches.insert(
                    *if_val as u64,
                    IfBranch {
                        value: *if_val as u64,
                        tokens,
                    },
                );
                // Parse ElseIf branches
                while let Some(Token::ElseIf(elif_val)) = iter.peek() {
                    iter.next();
                    let (elif_tokens, _has_endif) = parse_if_block_body(iter, error_list);
                    branches.insert(
                        *elif_val as u64,
                        IfBranch {
                            value: *elif_val as u64,
                            tokens: elif_tokens,
                        },
                    );
                }
                // Check for unmatched ElseIf
                if let Some(Token::ElseIf(_)) = iter.peek() {
                    error_list.push(ControlFlowRule::UnmatchedElseIf);
                    iter.next();
                }
                // Parse Else branch
                if let Some(Token::Else) = iter.peek() {
                    iter.next();
                    let (etokens, _has_endif) = parse_if_block_body(iter, error_list);
                    branches.insert(
                        0,
                        IfBranch {
                            value: 0,
                            tokens: etokens,
                        },
                    );
                }
                // Check for unmatched Else
                if let Some(Token::Else) = iter.peek() {
                    error_list.push(ControlFlowRule::UnmatchedElse);
                    iter.next();
                }
                if_blocks.push(IfBlock { branches });
            }
            Token::EndRandom => {
                iter.next();
                break;
            }
            Token::EndIf => {
                error_list.push(ControlFlowRule::UnmatchedEndIf);
                iter.next();
            }
            Token::EndSwitch => {
                error_list.push(ControlFlowRule::UnmatchedEndSwitch);
                iter.next();
            }
            // Automatically complete EndRandom: break when encountering Switch/SetSwitch/Case/Def/EndSwitch
            Token::SetSwitch(_) | Token::Switch(_) | Token::Case(_) | Token::Def | Token::Skip => {
                break;
            }
            _ => {
                iter.next();
            }
        }
    }
    // If the iterator has ended, also break (i.e., automatically complete EndRandom)
    Unit::RandomBlock {
        value: block_value,
        if_blocks,
    }
}

/// Parse the body of an If/ElseIf/Else block until EndIf/ElseIf/Else/EndRandom/EndSwitch
fn parse_if_block_body<'a, I>(
    iter: &mut std::iter::Peekable<I>,
    error_list: &mut Vec<ControlFlowRule>,
) -> (Vec<Unit<'a>>, bool)
where
    I: Iterator<Item = &'a Token<'a>>,
{
    let mut result = Vec::new();
    let mut has_endif = false;
    while let Some(token) = iter.peek() {
        match token {
            Token::ElseIf(_) | Token::Else | Token::EndIf | Token::EndRandom | Token::EndSwitch => {
                if let Token::EndIf = token {
                    has_endif = true;
                    iter.next();
                }
                break;
            }
            Token::If(_) => {
                result.push(parse_random_block(iter, error_list));
            }
            Token::SetSwitch(_) | Token::Switch(_) => {
                result.push(parse_switch_block(iter, error_list));
            }
            Token::Random(_) | Token::SetRandom(_) => {
                result.push(parse_random_block(iter, error_list));
            }
            _ => {
                if !token.is_control_flow_token() {
                    result.push(Unit::Token(*token));
                }
                iter.next();
            }
        }
    }
    (result, has_endif)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bms::lex::token::Token;

    #[test]
    fn test_switch_ast() {
        let tokens = vec![
            Token::SetSwitch(2),
            Token::Def,
            Token::Title("Out"),
            Token::Case(2),
            Token::Title("In 1"),
            Token::Case(1),
            Token::Title("In 2"),
            Token::Skip,
            Token::Case(3),
            Token::Title("In 3"),
            Token::Skip,
            Token::EndSwitch,
        ];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let ast = build_control_flow_ast(&stream, &mut errors);
        assert!(matches!(&ast[0], Unit::SwitchBlock { .. }));
        let Unit::SwitchBlock {
            value: BlockValue::Set { value: _ },
            cases,
        } = &ast[0]
        else {
            panic!("AST structure error");
        };
        assert_eq!(cases.len(), 4);
        assert!(matches!(cases[0].value, CaseBranchValue::Def));
        assert!(matches!(cases[1].value, CaseBranchValue::Case(2)));
        assert!(matches!(cases[2].value, CaseBranchValue::Case(1)));
        assert!(matches!(cases[3].value, CaseBranchValue::Case(3)));
        assert!(matches!(
            cases[0].tokens[0],
            Unit::Token(Token::Title("Out"))
        ));
        assert!(matches!(
            cases[1].tokens[0],
            Unit::Token(Token::Title("In 1"))
        ));
        assert!(matches!(
            cases[2].tokens[0],
            Unit::Token(Token::Title("In 2"))
        ));
        assert!(matches!(
            cases[3].tokens[0],
            Unit::Token(Token::Title("In 3"))
        ));
    }

    #[test]
    fn test_unmatched_endrandom_error() {
        use Token::*;
        let tokens = vec![Title("A"), EndRandom];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&stream, &mut errors);
        assert!(errors.contains(&ControlFlowRule::UnmatchedEndRandom));
    }

    #[test]
    fn test_unmatched_endif_error() {
        use Token::*;
        let tokens = vec![Title("A"), EndIf];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&stream, &mut errors);
        assert!(errors.contains(&ControlFlowRule::UnmatchedEndIf));
    }

    #[test]
    fn test_random_ast() {
        use Token::*;
        let tokens = vec![
            Random(2),
            If(1),
            Title("A"),
            EndIf,
            If(2),
            Title("B"),
            EndIf,
            EndRandom,
        ];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let ast = build_control_flow_ast(&stream, &mut errors);
        assert!(matches!(&ast[0], Unit::RandomBlock { .. }));
        let Unit::RandomBlock {
            value: _,
            if_blocks,
        } = &ast[0]
        else {
            panic!("AST structure error");
        };
        assert_eq!(if_blocks.len(), 2);
        let all_titles: Vec<_> = if_blocks
            .iter()
            .flat_map(|blk| blk.branches.values())
            .flat_map(|b| &b.tokens)
            .collect();
        assert!(
            all_titles
                .iter()
                .any(|u| matches!(u, Unit::Token(Token::Title("A"))))
        );
        assert!(
            all_titles
                .iter()
                .any(|u| matches!(u, Unit::Token(Token::Title("B"))))
        );
    }

    #[test]
    fn test_random_nested_ast() {
        use Token::*;
        let tokens = vec![
            Random(2),
            If(1),
            Title("A"),
            Random(2),
            If(2),
            Title("B"),
            EndIf,
            EndRandom,
            EndIf,
            EndRandom,
        ];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let ast = build_control_flow_ast(&stream, &mut errors);
        assert!(matches!(&ast[0], Unit::RandomBlock { .. }));
        let Unit::RandomBlock {
            value: _,
            if_blocks,
        } = &ast[0]
        else {
            panic!("AST structure error");
        };
        let mut found_nested = false;
        for blk in if_blocks {
            for branch in blk.branches.values() {
                if branch
                    .tokens
                    .iter()
                    .any(|u| matches!(u, Unit::RandomBlock { .. }))
                {
                    found_nested = true;
                }
            }
        }
        assert!(found_nested, "Nested RandomBlock not found");
    }

    #[test]
    fn test_random_multiple_if_elseif_else() {
        use Token::*;
        let tokens = vec![
            Random(3),
            If(1),
            Title("A1"),
            ElseIf(2),
            Title("A2"),
            Else,
            Title("Aelse"),
            EndIf,
            If(1),
            Title("B1"),
            ElseIf(2),
            Title("B2"),
            Else,
            Title("Belse"),
            EndIf,
            EndRandom,
        ];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let ast = build_control_flow_ast(&stream, &mut errors);
        assert!(matches!(&ast[0], Unit::RandomBlock { .. }));
        let Unit::RandomBlock {
            value: _,
            if_blocks,
        } = &ast[0]
        else {
            panic!("AST structure error");
        };
        assert_eq!(if_blocks.len(), 2);
        let branches1 = &if_blocks[0].branches;
        assert!(
            branches1
                .get(&1)
                .unwrap()
                .tokens
                .iter()
                .any(|u| matches!(u, Unit::Token(Token::Title("A1"))))
        );
        assert!(
            branches1
                .get(&2)
                .unwrap()
                .tokens
                .iter()
                .any(|u| matches!(u, Unit::Token(Token::Title("A2"))))
        );
        assert!(
            branches1
                .get(&0)
                .unwrap()
                .tokens
                .iter()
                .any(|u| matches!(u, Unit::Token(Token::Title("Aelse"))))
        );
        let branches2 = &if_blocks[1].branches;
        assert!(
            branches2
                .get(&1)
                .unwrap()
                .tokens
                .iter()
                .any(|u| matches!(u, Unit::Token(Token::Title("B1"))))
        );
        assert!(
            branches2
                .get(&2)
                .unwrap()
                .tokens
                .iter()
                .any(|u| matches!(u, Unit::Token(Token::Title("B2"))))
        );
        assert!(
            branches2
                .get(&0)
                .unwrap()
                .tokens
                .iter()
                .any(|u| matches!(u, Unit::Token(Token::Title("Belse"))))
        );
    }
}
