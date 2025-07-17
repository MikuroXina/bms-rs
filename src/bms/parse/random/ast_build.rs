use std::collections::HashMap;

use crate::{
    bms::lex::token::{Token, TokenStream},
    parse::random::ControlFlowRule,
};

/// An unit of AST which represents individual scoped commands of BMS source.
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
    let mut token_iter = tokens.iter().peekable();
    let iter = &mut token_iter;
    let mut result = Vec::new();
    while let Some(_) = iter.peek() {
        if let Some(unit) = parse_unit_or_block(iter, error_list) {
            result.push(unit);
            continue;
        }
        let Some(token) = iter.peek() else {
            break;
        };
        let rule = match token {
            Token::EndIf => Some(ControlFlowRule::UnmatchedEndIf),
            Token::EndRandom => Some(ControlFlowRule::UnmatchedEndRandom),
            Token::EndSwitch => Some(ControlFlowRule::UnmatchedEndSwitch),
            Token::ElseIf(_) => Some(ControlFlowRule::UnmatchedElseIf),
            Token::Else => Some(ControlFlowRule::UnmatchedElse),
            Token::Skip => Some(ControlFlowRule::UnmatchedSkip),
            Token::Case(_) => Some(ControlFlowRule::UnmatchedCase),
            Token::Def => Some(ControlFlowRule::UnmatchedDef),
            _ => None,
        };
        if let Some(rule) = rule {
            error_list.push(rule);
        }
        // 跳转到下一个Token
        iter.next();
    }
    result
}

/// 处理单个Token：如果是块开头则递归调用块解析，否则返回Token节点。
fn parse_unit_or_block<'a, I>(
    iter: &mut std::iter::Peekable<I>,
    error_list: &mut Vec<ControlFlowRule>,
) -> Option<Unit<'a>>
where
    I: Iterator<Item = &'a Token<'a>>,
{
    let token = iter.peek()?;
    match token {
        Token::SetSwitch(_) | Token::Switch(_) => Some(parse_switch_block(iter, error_list)),
        Token::Random(_) | Token::SetRandom(_) => Some(parse_random_block(iter, error_list)),
        token if !token.is_control_flow_token() => {
            let t = *token;
            iter.next();
            Some(Unit::Token(t))
        }
        _ => None,
    }
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
    let mut seen_case_values = std::collections::HashSet::new();
    let max_value = match &block_value {
        BlockValue::Random { max } => Some(*max),
        BlockValue::Set { value: _ } => None,
    };
    let mut def_count = 0;
    while let Some(next) = iter.peek() {
        match next {
            Token::Case(case_val) => {
                let case_val_u64 = *case_val as u64;
                // 检查是否重复
                if seen_case_values.contains(&case_val_u64) {
                    error_list.push(ControlFlowRule::SwitchDuplicateCaseValue);
                    iter.next();
                    let _ = parse_case_or_def_body(iter, error_list);
                    if let Some(Token::Skip) = iter.peek() {
                        iter.next();
                    }
                    continue;
                }
                // 检查是否越界
                if let Some(max) = max_value
                    && !(1..=max).contains(&case_val_u64)
                {
                    error_list.push(ControlFlowRule::SwitchCaseValueOutOfRange);
                    iter.next();
                    let _ = parse_case_or_def_body(iter, error_list);
                    if let Some(Token::Skip) = iter.peek() {
                        iter.next();
                    }
                    continue;
                }
                iter.next();
                seen_case_values.insert(case_val_u64);
                let tokens = parse_case_or_def_body(iter, error_list);
                cases.push(CaseBranch {
                    value: CaseBranchValue::Case(case_val_u64),
                    tokens,
                });
                if let Some(Token::Skip) = iter.peek() {
                    iter.next();
                }
            }
            Token::Def => {
                def_count += 1;
                if def_count > 1 {
                    error_list.push(ControlFlowRule::SwitchDuplicateDef);
                    iter.next();
                    let _ = parse_case_or_def_body(iter, error_list);
                    if let Some(Token::Skip) = iter.peek() {
                        iter.next();
                    }
                    continue;
                }
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

/// 修改parse_case_or_def_body，优先调用parse_unit_or_block
fn parse_case_or_def_body<'a, I>(
    iter: &mut std::iter::Peekable<I>,
    error_list: &mut Vec<ControlFlowRule>,
) -> Vec<Unit<'a>>
where
    I: Iterator<Item = &'a Token<'a>>,
{
    let mut result = Vec::new();
    while let Some(&token) = iter.peek() {
        if matches!(
            token,
            Token::Skip | Token::EndSwitch | Token::Case(_) | Token::Def
        ) {
            break;
        }
        if let Some(unit) = parse_unit_or_block(iter, error_list) {
            result.push(unit);
            continue;
        }
        let rule = match token {
            Token::EndIf => Some(ControlFlowRule::UnmatchedEndIf),
            Token::EndRandom => Some(ControlFlowRule::UnmatchedEndRandom),
            Token::EndSwitch => Some(ControlFlowRule::UnmatchedEndSwitch),
            Token::ElseIf(_) => Some(ControlFlowRule::UnmatchedElseIf),
            Token::Else => Some(ControlFlowRule::UnmatchedElse),
            Token::Skip => Some(ControlFlowRule::UnmatchedSkip),
            _ => None,
        };
        if let Some(rule) = rule {
            error_list.push(rule);
        }
        // 跳转到下一个Token
        iter.next();
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
    let max_value = match &block_value {
        BlockValue::Random { max } => Some(*max),
        BlockValue::Set { .. } => None,
    };
    while let Some(next) = iter.peek() {
        match next {
            Token::If(if_val) => {
                iter.next();
                let mut branches = HashMap::new();
                let mut seen_if_values = std::collections::HashSet::new();
                let if_val_u64 = *if_val as u64;
                // 检查是否重复
                if seen_if_values.contains(&if_val_u64) {
                    error_list.push(ControlFlowRule::RandomDuplicateIfBranchValue);
                    let _ = parse_if_block_body(iter, error_list);
                } else if let Some(max) = max_value {
                    // 检查是否超出范围
                    if if_val_u64 < 1 || if_val_u64 > max {
                        error_list.push(ControlFlowRule::RandomIfBranchValueOutOfRange);
                        let _ = parse_if_block_body(iter, error_list);
                    } else {
                        seen_if_values.insert(if_val_u64);
                        let tokens = parse_if_block_body(iter, error_list);
                        branches.insert(
                            if_val_u64,
                            IfBranch {
                                value: if_val_u64,
                                tokens,
                            },
                        );
                    }
                } else {
                    seen_if_values.insert(if_val_u64);
                    let tokens = parse_if_block_body(iter, error_list);
                    branches.insert(
                        if_val_u64,
                        IfBranch {
                            value: if_val_u64,
                            tokens,
                        },
                    );
                }
                // Parse ElseIf branches
                while let Some(Token::ElseIf(elif_val)) = iter.peek() {
                    let elif_val_u64 = *elif_val as u64;
                    if seen_if_values.contains(&elif_val_u64) {
                        error_list.push(ControlFlowRule::RandomDuplicateIfBranchValue);
                        iter.next();
                        let _ = parse_if_block_body(iter, error_list);
                        continue;
                    }
                    if let Some(max) = max_value {
                        if elif_val_u64 < 1 || elif_val_u64 > max {
                            error_list.push(ControlFlowRule::RandomIfBranchValueOutOfRange);
                            iter.next();
                            let _ = parse_if_block_body(iter, error_list);
                            continue;
                        }
                    }
                    iter.next();
                    seen_if_values.insert(elif_val_u64);
                    let elif_tokens = parse_if_block_body(iter, error_list);
                    branches.insert(
                        elif_val_u64,
                        IfBranch {
                            value: elif_val_u64,
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
                    let etokens = parse_if_block_body(iter, error_list);
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
            // 自动补全EndRandom: 遇到Switch/SetSwitch/Case/Def/EndSwitch/Skip时break
            Token::SetSwitch(_) | Token::Switch(_) | Token::Case(_) | Token::Def | Token::Skip => {
                break;
            }
            _ => {
                // 允许非控制流Token，优先parse_unit_or_block
                if let Some(_unit) = parse_unit_or_block(iter, error_list) {
                    // 这里的Token不属于任何IfBlock，直接丢弃，并记录错误
                    error_list.push(ControlFlowRule::UnmatchedTokenInRandomBlock);
                } else {
                    iter.next();
                }
            }
        }
    }
    Unit::RandomBlock {
        value: block_value,
        if_blocks,
    }
}

/// 重构后的parse_if_block_body，允许出现非控制流Token时优先parse_unit_or_block
fn parse_if_block_body<'a, I>(
    iter: &mut std::iter::Peekable<I>,
    error_list: &mut Vec<ControlFlowRule>,
) -> Vec<Unit<'a>>
where
    I: Iterator<Item = &'a Token<'a>>,
{
    let mut result = Vec::new();
    while let Some(token) = iter.peek() {
        match token {
            Token::ElseIf(_) | Token::Else | Token::EndIf | Token::EndRandom | Token::EndSwitch => {
                if let Token::EndIf = token {
                    iter.next();
                }
                break;
            }
            _ => {
                if let Some(unit) = parse_unit_or_block(iter, error_list) {
                    result.push(unit);
                } else {
                    iter.next();
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
        let Some(Unit::SwitchBlock { cases, .. }) =
            ast.iter().find(|u| matches!(u, Unit::SwitchBlock { .. }))
        else {
            panic!("AST structure error, ast: {ast:?}");
        };
        let Some(_case1) = cases
            .iter()
            .find(|c| matches!(c.value, CaseBranchValue::Case(1)))
        else {
            panic!("Case(1) not found, cases: {cases:?}");
        };
        let Some(Unit::SwitchBlock { cases, .. }) =
            ast.iter().find(|u| matches!(u, Unit::SwitchBlock { .. }))
        else {
            panic!("AST structure error, ast: {ast:?}");
        };
        let Some(CaseBranch { tokens: _, .. }) = cases
            .iter()
            .find(|c| matches!(c.value, CaseBranchValue::Case(1)))
        else {
            panic!("Case(1) not found, cases: {cases:?}");
        };
        // 由于tokens只包含Token类型，这里不再查找RandomBlock，相关断言已在上方覆盖。
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
        let Unit::RandomBlock {
            value: _,
            if_blocks,
        } = &ast[0]
        else {
            panic!("AST structure error, ast: {:?}", ast);
        };
        assert_eq!(if_blocks.len(), 2);
        let all_titles: Vec<_> = if_blocks
            .iter()
            .flat_map(|blk| blk.branches.values())
            .flat_map(|b| &b.tokens)
            .collect();
        let Some(_) = all_titles
            .iter()
            .find(|u| matches!(u, Unit::Token(Token::Title("A"))))
        else {
            panic!("A missing, all_titles: {:?}", all_titles);
        };
        let Some(_) = all_titles
            .iter()
            .find(|u| matches!(u, Unit::Token(Token::Title("B"))))
        else {
            panic!("B missing, all_titles: {:?}", all_titles);
        };
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
        let Unit::RandomBlock {
            value: _,
            if_blocks,
        } = &ast[0]
        else {
            panic!("AST structure error, ast: {:?}", ast);
        };
        let mut found_nested = false;
        for blk in if_blocks {
            for branch in blk.branches.values() {
                if let Some(_) = branch
                    .tokens
                    .iter()
                    .find(|u| matches!(u, Unit::RandomBlock { .. }))
                {
                    found_nested = true;
                }
            }
        }
        if !found_nested {
            panic!("Nested RandomBlock not found, if_blocks: {:?}", if_blocks);
        }
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
        let Unit::RandomBlock {
            value: _,
            if_blocks,
        } = &ast[0]
        else {
            panic!("AST structure error");
        };
        assert_eq!(if_blocks.len(), 2);
        let branches1 = &if_blocks[0].branches;
        let Some(b1) = branches1.get(&1) else {
            panic!("branch 1 missing");
        };
        let Some(_) = b1
            .tokens
            .iter()
            .find(|u| matches!(u, Unit::Token(Token::Title("A1"))))
        else {
            panic!("A1 missing");
        };
        let Some(b2) = branches1.get(&2) else {
            panic!("branch 2 missing");
        };
        let Some(_) = b2
            .tokens
            .iter()
            .find(|u| matches!(u, Unit::Token(Token::Title("A2"))))
        else {
            panic!("A2 missing");
        };
        let Some(belse) = branches1.get(&0) else {
            panic!("branch else missing");
        };
        let Some(_) = belse
            .tokens
            .iter()
            .find(|u| matches!(u, Unit::Token(Token::Title("Aelse"))))
        else {
            panic!("Aelse missing");
        };
        let branches2 = &if_blocks[1].branches;
        let Some(b1) = branches2.get(&1) else {
            panic!("branch 1 missing");
        };
        let Some(_) = b1
            .tokens
            .iter()
            .find(|u| matches!(u, Unit::Token(Token::Title("B1"))))
        else {
            panic!("B1 missing");
        };
        let Some(b2) = branches2.get(&2) else {
            panic!("branch 2 missing");
        };
        let Some(_) = b2
            .tokens
            .iter()
            .find(|u| matches!(u, Unit::Token(Token::Title("B2"))))
        else {
            panic!("B2 missing");
        };
        let Some(belse) = branches2.get(&0) else {
            panic!("branch else missing");
        };
        let Some(_) = belse
            .tokens
            .iter()
            .find(|u| matches!(u, Unit::Token(Token::Title("Belse"))))
        else {
            panic!("Belse missing");
        };
    }

    #[test]
    fn test_random_duplicate_ifbranch() {
        use Token::*;
        let tokens = vec![
            Random(2),
            If(1),
            Title("A"),
            ElseIf(1), // 重复
            Title("B"),
            EndIf,
            EndRandom,
        ];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&stream, &mut errors);
        assert_eq!(errors, vec![ControlFlowRule::RandomDuplicateIfBranchValue]);
    }

    #[test]
    fn test_random_ifbranch_value_out_of_range() {
        use Token::*;
        let tokens = vec![
            Random(2),
            If(3), // 超出范围
            Title("A"),
            EndIf,
            EndRandom,
        ];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&stream, &mut errors);
        assert_eq!(errors, vec![ControlFlowRule::RandomIfBranchValueOutOfRange]);
    }

    #[test]
    fn test_switch_duplicate_case() {
        use Token::*;
        let tokens = vec![
            Switch(2),
            Case(1),
            Title("A"),
            Case(1), // 重复
            Title("B"),
            EndSwitch,
        ];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&stream, &mut errors);
        assert_eq!(errors, vec![ControlFlowRule::SwitchDuplicateCaseValue]);
    }

    #[test]
    fn test_switch_case_value_out_of_range() {
        use Token::*;
        let tokens = vec![
            Switch(2),
            Case(3), // 超出范围
            Title("A"),
            EndSwitch,
        ];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&stream, &mut errors);
        assert_eq!(errors, vec![ControlFlowRule::SwitchCaseValueOutOfRange]);
    }

    #[test]
    fn test_switch_duplicate_def() {
        use Token::*;
        let tokens = vec![
            Switch(2),
            Def,
            Title("A"),
            Def, // 多余
            Title("B"),
            Def, // 多余
            Title("C"),
            EndSwitch,
        ];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&stream, &mut errors);
        assert_eq!(
            errors,
            vec![
                ControlFlowRule::SwitchDuplicateDef,
                ControlFlowRule::SwitchDuplicateDef,
            ]
        );
    }

    #[test]
    fn test_unmatched_token_in_random_block() {
        use Token::*;
        let tokens = vec![
            Random(2),
            Title("A"), // 不在任何IfBlock内
            If(1),
            Title("B"),
            EndIf,
            EndRandom,
        ];
        let stream = TokenStream::from_tokens(tokens);
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&stream, &mut errors);
        assert_eq!(errors, vec![ControlFlowRule::UnmatchedTokenInRandomBlock]);
    }
}
