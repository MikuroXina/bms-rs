use std::collections::HashMap;

use thiserror::Error;

use super::{ParseError, rng::Rng};
use crate::bms::lex::token::{Token, TokenStream};

/// Parses the control flow of the token.
/// Returns the tokens that will be executed, and not contains control flow tokens.
pub(super) fn parse_control_flow<'a>(
    token_stream: &'a TokenStream<'a>,
    mut rng: impl Rng,
) -> Result<Vec<&'a Token<'a>>, ParseError> {
    let mut error_list = Vec::new();
    let ast: Vec<Unit<'a>> = build_control_flow_ast(token_stream, &mut error_list);
    let mut ast_iter = ast.into_iter().peekable();
    let tokens: Vec<&'a Token<'a>> =
        parse_control_flow_ast(&mut ast_iter, &mut rng, &mut error_list);
    match error_list.into_iter().next() {
        Some(error) => Err(error.into()),
        None => Ok(tokens),
    }
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

fn build_control_flow_ast<'a>(
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
            // 自动补全EndSwitch：遇到Random/SetRandom/If/EndRandom/EndIf时自动break
            Token::Random(_) | Token::SetRandom(_) | Token::If(_) => {
                break;
            }
            _ => {
                iter.next();
            }
        }
    }
    // 如果迭代器已经结束，也自动break（即自动补全EndSwitch）
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
                // 检查未匹配的ElseIf
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
                // 检查未匹配的Else
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
            // 自动补全EndRandom：遇到Switch/SetSwitch/Case/Def/EndSwitch时自动break
            Token::SetSwitch(_) | Token::Switch(_) | Token::Case(_) | Token::Def | Token::Skip => {
                break;
            }
            _ => {
                iter.next();
            }
        }
    }
    // 如果迭代器已经结束，也自动break（即自动补全EndRandom）
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

fn parse_control_flow_ast<'a>(
    iter: &mut std::iter::Peekable<impl Iterator<Item = Unit<'a>>>,
    rng: &mut impl Rng,
    error_list: &mut Vec<ControlFlowRule>,
) -> Vec<&'a Token<'a>> {
    let mut result = Vec::new();
    while let Some(unit) = iter.next() {
        match unit {
            Unit::Token(token) => {
                result.push(token);
            }
            Unit::RandomBlock { value, if_blocks } => {
                // 选择分支
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
                // 查找第一个if_block中包含该分支的分支
                let mut found = false;
                for if_block in &if_blocks {
                    if let Some(branch) = if_block.branches.get(&branch_val) {
                        let mut branch_iter = branch.tokens.clone().into_iter().peekable();
                        result.extend(parse_control_flow_ast(&mut branch_iter, rng, error_list));
                        found = true;
                        break;
                    }
                }
                // 如果没有找到，尝试找0（else）分支
                if !found {
                    for if_block in &if_blocks {
                        #[allow(unused_assignments)]
                        if let Some(branch) = if_block.branches.get(&0) {
                            let mut branch_iter = branch.tokens.clone().into_iter().peekable();
                            result.extend(parse_control_flow_ast(
                                &mut branch_iter,
                                rng,
                                error_list,
                            ));
                            found = true;
                            break;
                        }
                    }
                }
                // 如果都没有，什么都不做
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
                // 查找Case分支
                let mut found = false;
                for case in &cases {
                    match &case.value {
                        CaseBranchValue::Case(val) if *val == switch_val => {
                            let mut case_iter = case.tokens.clone().into_iter().peekable();
                            result.extend(parse_control_flow_ast(&mut case_iter, rng, error_list));
                            found = true;
                            break;
                        }
                        _ => {}
                    }
                }
                // 如果没有Case匹配，找Def分支
                if !found {
                    for case in &cases {
                        if let CaseBranchValue::Def = case.value {
                            let mut case_iter = case.tokens.clone().into_iter().peekable();
                            result.extend(parse_control_flow_ast(&mut case_iter, rng, error_list));
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
            // 总是返回最大值
            *_range.end()
        }
    }

    #[test]
    fn test_setrandom_setwitch_large_value() {
        // SetRandom/SetSwitch下If/Case值超大
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
        let mut errors = Vec::new();
        let mut iter = units.into_iter().peekable();
        let tokens = parse_control_flow_ast(&mut iter, &mut rng, &mut errors);
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
        // 嵌套Random和Switch，互相嵌套
        let mut rng = DummyRng;
        let mut errors = Vec::new();
        // Random外层，Switch内层
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

        // Switch外层，Random内层
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
        // 多层嵌套Random和Switch
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

    #[test]
    fn test_switch_nested_switch_case() {
        use Token::*;
        let tokens = vec![
            Title("11000000"),
            Switch(2),
            Case(1),
            Title("00220000"),
            Random(2),
            If(1),
            Title("00550000"),
            ElseIf(2),
            Title("00006600"),
            EndIf,
            EndRandom,
            Skip,
            Case(2),
            Title("00003300"),
            Skip,
            EndSwitch,
            Title("00000044"),
        ];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let ast = build_control_flow_ast(&stream, &mut errors);
        println!("AST structure: {:#?}", ast);
        let Some(Unit::SwitchBlock { cases, .. }) =
            ast.iter().find(|u| matches!(u, Unit::SwitchBlock { .. }))
        else {
            panic!("AST structure error");
        };
        let Some(case1) = cases
            .iter()
            .find(|c| matches!(c.value, CaseBranchValue::Case(1)))
        else {
            panic!("Case(1) not found");
        };
        println!("Case(1) tokens: {:#?}", case1.tokens);
        assert_eq!(errors, vec![]);
        assert!(matches!(&ast[0], Unit::Token(_))); // 11000000
        assert!(matches!(&ast[1], Unit::SwitchBlock { .. }));
        assert!(matches!(&ast[2], Unit::Token(_))); // 00000044
        let Unit::SwitchBlock { cases, .. } = &ast[1] else {
            panic!("AST structure error");
        };
        let Some(CaseBranch { tokens, .. }) = cases
            .iter()
            .find(|c| matches!(c.value, CaseBranchValue::Case(1)))
        else {
            panic!("Case(1) not found");
        };
        assert!(matches!(&tokens[0], Unit::Token(_))); // 00220000
        assert!(matches!(&tokens[1], Unit::RandomBlock { .. }));
        let Unit::RandomBlock { if_blocks, .. } = &tokens[1] else {
            panic!("RandomBlock not found");
        };
        let if_block = &if_blocks[0];
        assert!(
            if_block
                .branches
                .get(&1)
                .unwrap()
                .tokens
                .iter()
                .any(|u| matches!(u, Unit::Token(Title("00550000"))))
        );
        assert!(
            if_block
                .branches
                .get(&2)
                .unwrap()
                .tokens
                .iter()
                .any(|u| matches!(u, Unit::Token(Title("00006600"))))
        );
        let Some(CaseBranch { tokens, .. }) = cases
            .iter()
            .find(|c| matches!(c.value, CaseBranchValue::Case(2)))
        else {
            panic!("Case(2) not found");
        };
        assert!(matches!(&tokens[0], Unit::Token(Title("00003300"))));
        let mut rng = DummyRng;
        let mut errors2 = Vec::new();
        let mut ast_iter = ast.into_iter().peekable();
        let tokens = parse_control_flow_ast(&mut ast_iter, &mut rng, &mut errors2);
        let expected = ["11000000", "00003300", "00000044"];
        assert_eq!(tokens.len(), 3);
        for (i, t) in tokens.iter().enumerate() {
            match t {
                Title(s) => {
                    assert_eq!(s, &expected[i], "Title content mismatch");
                }
                _ => panic!("Token type mismatch"),
            }
        }
        assert_eq!(errors, vec![]);
        assert_eq!(errors2, vec![]);
    }

    #[test]
    fn test_switch_insane_tokenized() {
        use Token::*;
        let tokens = vec![
            Switch(5),
            Def,
            Title("0055"),
            Skip,
            Case(1),
            Title("0100000000000000"),
            Random(2),
            If(1),
            Title("04"),
            Else,
            Title("05"),
            EndIf,
            // Missing EndRandom!!!
            Case(2),
            Title("0200000000000000"),
            Skip,
            Case(3),
            Title("0300000000000000"),
            Switch(2),
            Case(1),
            Title("1111"),
            Skip,
            Case(2),
            Title("2222"),
            Skip,
            EndSwitch,
            Skip,
            EndSwitch,
        ];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let ast = build_control_flow_ast(&stream, &mut errors);
        println!("AST structure: {:#?}", ast);
        let Some(Unit::SwitchBlock { cases, .. }) =
            ast.iter().find(|u| matches!(u, Unit::SwitchBlock { .. }))
        else {
            panic!("AST structure error");
        };
        let Some(case1) = cases
            .iter()
            .find(|c| matches!(c.value, CaseBranchValue::Case(1)))
        else {
            panic!("Case(1) not found");
        };
        println!("Case(1) tokens: {:#?}", case1.tokens);
        let mut rng = DummyRng;
        let mut errors2 = Vec::new();
        let mut ast_iter = ast.clone().into_iter().peekable();
        let _tokens = parse_control_flow_ast(&mut ast_iter, &mut rng, &mut errors2);
        let mut rng = DummyRng;
        let mut errors3 = Vec::new();
        let mut ast_iter = ast.into_iter().peekable();
        let _tokens = parse_control_flow_ast(&mut ast_iter, &mut rng, &mut errors3);
        assert_eq!(errors, vec![]);
        assert_eq!(errors2, vec![]);
        assert_eq!(errors3, vec![]);
    }
}
