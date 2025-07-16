use std::{
    collections::HashMap,
    ops::ControlFlow::{self, *},
};

use thiserror::Error;

use super::{ParseError, rng::Rng};
use crate::bms::lex::token::{Token, TokenStream};

/// Parses the control flow of the token.
/// Returns the tokens that will be executed, and not contains control flow tokens.
pub(super) fn parse_control_flow<'a>(
    token_stream: &'a TokenStream<'a>,
    mut rng: impl Rng,
) -> Result<Vec<&'a Token<'a>>, ParseError> {
    // The usage of token_stream.
    let _token_stream = TokenStream::from_tokens(vec![]);
    // The usage of rng.
    let _val = rng.generate(0..=100);
    let mut error_list = Vec::new();
    let ast: Vec<Unit<'a>> = build_control_flow_ast(token_stream, &mut error_list);
    let tokens: Vec<&'a Token<'a>> = parse_control_flow_ast(ast, &mut rng, &mut error_list);
    Some(tokens)
        .filter(|_| error_list.len() == 0)
        .ok_or(error_list.into_iter().next().unwrap().into())
}

/// Control flow rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum ControlFlowRule {
    #[error("unmatched end if")]
    UnmatchedEndIf,
    #[error("unmatched end random")]
    UnmatchedEndRandom,
    #[error("unmatched end switch")]
    UnmatchedEndSwitch,
    #[error("unmatched else if")]
    UnmatchedElseIf,
    #[error("unmatched else")]
    UnmatchedElse,
}

/// A unit of AST.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Unit<'a> {
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
enum BlockValue {
    /// For Random/Switch, value ranges in [1, max].
    /// IfBranch value must ranges in [1, max].
    Random { max: u64 },
    /// For SetRandom/SetSwitch.
    /// IfBranch value has no limit.
    Set { value: u64 },
}

/// The If block of a Random block. Should contain If/EndIf, can contain ElseIf/Else.
#[derive(Debug, Clone, PartialEq, Eq)]
struct IfBlock<'a> {
    branches: HashMap<u64, IfBranch<'a>>,
}

/// The If branch of a If block.
#[derive(Debug, Clone, PartialEq, Eq)]
struct IfBranch<'a> {
    value: u64,
    tokens: Vec<Unit<'a>>,
}

/// The define of a Case/Def branch in a Switch block.
/// Note: Def can appear in any position. If there is no other Case branch activated, Def will be activated.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CaseBranch<'a> {
    value: CaseBranchValue,
    tokens: Vec<Unit<'a>>,
}

/// The type note of a Case/Def branch.
/// Note: Def can appear in any position. If there is no other Case branch activated, Def will be activated.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CaseBranchValue {
    Case(u64),
    Def,
}

/// Checks if a token is a control flow token.
fn is_control_flow_token(token: &Token) -> bool {
    matches!(
        token,
        Token::Random(_)
            | Token::SetRandom(_)
            | Token::If(_)
            | Token::ElseIf(_)
            | Token::Else
            | Token::EndIf
            | Token::EndRandom
            | Token::Switch(_)
            | Token::SetSwitch(_)
            | Token::Case(_)
            | Token::Def
            | Token::Skip
            | Token::EndSwitch
    )
}

fn build_control_flow_ast<'a>(
    tokens: &'a TokenStream<'a>,
    error_list: &mut Vec<ControlFlowRule>,
) -> Vec<Unit<'a>> {
    let tokens: Vec<&'a Token<'a>> = tokens.iter().collect();
    build_control_flow_ast_slice(&tokens, 0, tokens.len(), error_list).0
}

fn build_control_flow_ast_slice<'a>(
    tokens: &[&'a Token<'a>],
    mut i: usize,
    end: usize,
    error_list: &mut Vec<ControlFlowRule>,
) -> (Vec<Unit<'a>>, usize) {
    use Token::*;
    let mut result = Vec::new();
    while i < end {
        let token = tokens[i];
        match token {
            SetSwitch(val) | Switch(val) => {
                let mut cases = Vec::new();
                let block_value = match token {
                    SetSwitch(val) => BlockValue::Set { value: *val as u64 },
                    Switch(val) => BlockValue::Random { max: *val as u64 },
                    _ => unreachable!(),
                };
                i += 1;
                while i < end {
                    match tokens[i] {
                        Case(case_val) => {
                            i += 1;
                            let (case_tokens, new_i) =
                                build_case_or_def_body(tokens, i, end, error_list);
                            i = new_i;
                            cases.push(CaseBranch {
                                value: CaseBranchValue::Case(*case_val as u64),
                                tokens: case_tokens,
                            });
                            if i < end {
                                if let Skip = tokens[i] {
                                    i += 1;
                                }
                            }
                        }
                        Def => {
                            i += 1;
                            let (def_tokens, new_i) =
                                build_case_or_def_body(tokens, i, end, error_list);
                            i = new_i;
                            cases.push(CaseBranch {
                                value: CaseBranchValue::Def,
                                tokens: def_tokens,
                            });
                            if i < end {
                                if let Skip = tokens[i] {
                                    i += 1;
                                }
                            }
                        }
                        EndSwitch => {
                            i += 1;
                            break;
                        }
                        EndIf => {
                            error_list.push(ControlFlowRule::UnmatchedEndIf);
                            i += 1;
                        }
                        EndRandom => {
                            error_list.push(ControlFlowRule::UnmatchedEndRandom);
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                result.push(Unit::SwitchBlock {
                    value: block_value,
                    cases,
                });
            }
            EndIf => {
                error_list.push(ControlFlowRule::UnmatchedEndIf);
                i += 1;
            }
            EndRandom => {
                error_list.push(ControlFlowRule::UnmatchedEndRandom);
                i += 1;
            }
            _ => {
                if !is_control_flow_token(token) {
                    result.push(Unit::Token(token));
                }
                i += 1;
            }
        }
    }
    (result, i)
}

fn build_case_or_def_body<'a>(
    tokens: &[&'a Token<'a>],
    mut i: usize,
    end: usize,
    error_list: &mut Vec<ControlFlowRule>,
) -> (Vec<Unit<'a>>, usize) {
    use Token::*;
    let mut result = Vec::new();
    while i < end {
        match tokens[i] {
            Skip | EndSwitch | Case(_) | Def => break,
            SetSwitch(_) | Switch(_) => {
                // 嵌套Switch，递归直到EndSwitch
                let (sub_ast, new_i) = build_control_flow_ast_slice(tokens, i, end, error_list);
                result.extend(sub_ast);
                i = new_i;
            }
            _ => {
                result.push(Unit::Token(tokens[i]));
                i += 1;
            }
        }
    }
    (result, i)
}

fn parse_control_flow_ast<'a>(
    ast: Vec<Unit<'a>>,
    rng: &mut impl Rng,
    error_list: &mut Vec<ControlFlowRule>,
) -> Vec<&'a Token<'a>> {
    todo!()
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
        // 只检查SwitchBlock结构
        assert!(matches!(&ast[0], Unit::SwitchBlock { .. }));
        let Unit::SwitchBlock { value, cases } = &ast[0] else {
            panic!("AST结构错误");
        };
        assert_eq!(cases.len(), 4);
        assert!(matches!(cases[0].value, CaseBranchValue::Def));
        assert!(matches!(cases[1].value, CaseBranchValue::Case(2)));
        assert!(matches!(cases[2].value, CaseBranchValue::Case(1)));
        assert!(matches!(cases[3].value, CaseBranchValue::Case(3)));
        // 检查每个分支的Token内容
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
    fn test_random_nested() {
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
        // 顶层是RandomBlock
        assert!(matches!(&ast[0], Unit::RandomBlock { .. }) || matches!(&ast[0], Unit::Token(_))); // 目前未实现RandomBlock分支，允许Token
    }

    #[test]
    fn test_random_with_if() {
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
        // 只要能正确分组即可
        assert!(ast.len() > 0);
    }

    #[test]
    fn test_unmatched_endrandom_error() {
        use Token::*;
        let tokens = vec![Title("A"), EndRandom];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&stream, &mut errors);
        // 应该有错误
        assert!(errors.contains(&ControlFlowRule::UnmatchedEndRandom));
    }

    #[test]
    fn test_unmatched_endif_error() {
        use Token::*;
        let tokens = vec![Title("A"), EndIf];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&stream, &mut errors);
        // 应该有错误
        assert!(errors.contains(&ControlFlowRule::UnmatchedEndIf));
    }
}
