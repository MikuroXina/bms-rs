use std::collections::HashMap;

use crate::{bms::lex::token::Token, parse::random::ControlFlowRule};

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

/// The main entry for building the control flow AST. Traverses the Token stream and recursively parses all control flow blocks.
/// Returns a list of AST nodes and collects all control flow related errors.
pub(super) fn build_control_flow_ast<'a>(
    tokens_iter: &mut std::iter::Peekable<impl Iterator<Item = &'a Token<'a>>>,
    error_list: &mut Vec<ControlFlowRule>,
) -> Vec<Unit<'a>> {
    let mut result = Vec::new();
    while tokens_iter.peek().is_some() {
        if let Some(unit) = parse_unit_or_block(tokens_iter, error_list) {
            result.push(unit);
            continue;
        }
        let Some(token) = tokens_iter.peek() else {
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
        // Jump to the next Token
        tokens_iter.next();
    }
    result
}

/// Handle a single Token: if it is the start of a block, recursively call the block parser, otherwise return a Token node.
fn parse_unit_or_block<'a>(
    iter: &mut std::iter::Peekable<impl Iterator<Item = &'a Token<'a>>>,
    error_list: &mut Vec<ControlFlowRule>,
) -> Option<Unit<'a>> {
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

/// Parse a Switch/SetSwitch block until EndSwitch or auto-completion termination.
/// Supports Case/Def branches, error detection, and nested structures.
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
    let mut seen_def = false;
    while let Some(next) = iter.peek() {
        match next {
            Token::Case(case_val) => {
                let case_val_u64 = *case_val as u64;
                // Check for duplicates
                if seen_case_values.contains(&case_val_u64) {
                    error_list.push(ControlFlowRule::SwitchDuplicateCaseValue);
                    iter.next();
                    let _ = parse_case_or_def_body(iter, error_list);
                    if let Some(Token::Skip) = iter.peek() {
                        iter.next();
                    }
                    continue;
                }
                // Check for out-of-range
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
                if seen_def {
                    error_list.push(ControlFlowRule::SwitchDuplicateDef);
                    iter.next();
                    let _ = parse_case_or_def_body(iter, error_list);
                    if let Some(Token::Skip) = iter.peek() {
                        iter.next();
                    }
                    continue;
                }
                seen_def = true;
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

/// Parse the body of a Case/Def branch until a branch-terminating Token is encountered.
/// Supports nested blocks, prioritizing parse_unit_or_block.
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
        // Jump to the next Token
        iter.next();
    }
    result
}

/// Parse a Random/SetRandom block until EndRandom or auto-completion termination.
/// Supports nesting, error detection, and records errors when non-control-flow Tokens appear outside IfBlock.
/// Design:
/// - After entering Random/SetRandom, loop through Tokens.
/// - If encountering If/ElseIf/Else, collect branches and check for duplicates/out-of-range.
/// - If encountering a non-control-flow Token, prioritize parse_unit_or_block; if not in any IfBlock, report error.
/// - Supports nested structures; recursively handle other block types.
fn parse_random_block<'a, I>(
    iter: &mut std::iter::Peekable<I>,
    error_list: &mut Vec<ControlFlowRule>,
) -> Unit<'a>
where
    I: Iterator<Item = &'a Token<'a>>,
{
    // 1. Read the Random/SetRandom header to determine the max branch value
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
    // 2. Main loop, process the contents inside the Random block
    while let Some(next) = iter.peek() {
        match next {
            // 2.1 Handle If branch
            Token::If(if_val) => {
                iter.next();
                let mut branches = HashMap::new();
                let mut seen_if_values = std::collections::HashSet::new();
                let if_val_u64 = *if_val as u64;
                // Check if If branch value is duplicated
                if seen_if_values.contains(&if_val_u64) {
                    error_list.push(ControlFlowRule::RandomDuplicateIfBranchValue);
                    let _ = parse_if_block_body(iter, error_list);
                } else if let Some(max) = max_value {
                    // Check if If branch value is out-of-range
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
                    // SetRandom branch has no range limit
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
                // 2.2 Handle ElseIf branches, same logic as If
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
                // 2.3 Check for redundant ElseIf
                if let Some(Token::ElseIf(_)) = iter.peek() {
                    error_list.push(ControlFlowRule::UnmatchedElseIf);
                    iter.next();
                }
                // 2.4 Handle Else branch, branch value is 0
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
                // 2.5 Check for redundant Else
                if let Some(Token::Else) = iter.peek() {
                    error_list.push(ControlFlowRule::UnmatchedElse);
                    iter.next();
                }
                // 2.6 Collect this IfBlock
                if_blocks.push(IfBlock { branches });
            }
            // 3.1 Termination: EndRandom encountered, block ends
            Token::EndRandom => {
                iter.next();
                break;
            }
            // 3.2 Error: EndIf/EndSwitch encountered, record error and skip
            Token::EndIf => {
                error_list.push(ControlFlowRule::UnmatchedEndIf);
                iter.next();
            }
            Token::EndSwitch => {
                error_list.push(ControlFlowRule::UnmatchedEndSwitch);
                iter.next();
            }
            // 3.3 Auto-completion termination: break early when encountering other block headers or Case/Def/Skip
            Token::SetSwitch(_) | Token::Switch(_) | Token::Case(_) | Token::Def | Token::Skip => {
                break;
            }
            // 4. Handle non-control-flow Token: prioritize parse_unit_or_block; if not in any IfBlock, report error
            _ => {
                if let Some(_unit) = parse_unit_or_block(iter, error_list) {
                    // This Token does not belong to any IfBlock, discard directly and record error
                    error_list.push(ControlFlowRule::UnmatchedTokenInRandomBlock);
                } else {
                    iter.next();
                }
            }
        }
    }
    // 5. Return AST node
    Unit::RandomBlock {
        value: block_value,
        if_blocks,
    }
}

/// Parse the body of an If/ElseIf/Else branch until a branch-terminating Token is encountered.
/// Design:
/// - Supports nested blocks, prioritizing parse_unit_or_block.
/// - Break when encountering branch-terminating Tokens (ElseIf/Else/EndIf/EndRandom/EndSwitch).
/// - If EndIf is encountered, consume it automatically.
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
            // 1. Branch-terminating Token, break
            Token::ElseIf(_) | Token::Else | Token::EndIf | Token::EndRandom | Token::EndSwitch => {
                if let Token::EndIf = token {
                    iter.next(); // Automatically consume EndIf
                }
                break;
            }
            // 2. Other content, prioritize parse_unit_or_block (supports nesting)
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
        let mut errors = Vec::new();
        let ast = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
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
        // Since tokens only contain Token type, do not search for RandomBlock here. Related assertions are already covered above.
    }

    #[test]
    fn test_unmatched_endrandom_error() {
        use Token::*;
        let tokens = [Title("A"), EndRandom];
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
        assert!(errors.contains(&ControlFlowRule::UnmatchedEndRandom));
    }

    #[test]
    fn test_unmatched_endif_error() {
        use Token::*;
        let tokens = [Title("A"), EndIf];
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
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
        let mut errors = Vec::new();
        let ast = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
        let Unit::RandomBlock {
            value: _,
            if_blocks,
        } = &ast[0]
        else {
            panic!("AST structure error, ast: {ast:?}");
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
            panic!("A missing, all_titles: {all_titles:?}");
        };
        let Some(_) = all_titles
            .iter()
            .find(|u| matches!(u, Unit::Token(Token::Title("B"))))
        else {
            panic!("B missing, all_titles: {all_titles:?}");
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
        let mut errors = Vec::new();
        let ast = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
        let Unit::RandomBlock {
            value: _,
            if_blocks,
        } = &ast[0]
        else {
            panic!("AST structure error, ast: {ast:?}");
        };
        let mut found_nested = false;
        for blk in if_blocks {
            for branch in blk.branches.values() {
                if branch
                    .tokens
                    .iter()
                    .any(|u| matches!(&u, Unit::RandomBlock { .. }))
                {
                    found_nested = true;
                }
            }
        }
        if !found_nested {
            panic!("Nested RandomBlock not found, if_blocks: {if_blocks:?}");
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
        let mut errors = Vec::new();
        let ast = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
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
            ElseIf(1), // duplicate
            Title("B"),
            EndIf,
            EndRandom,
        ];
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
        assert_eq!(errors, vec![ControlFlowRule::RandomDuplicateIfBranchValue]);
    }

    #[test]
    fn test_random_ifbranch_value_out_of_range() {
        use Token::*;
        let tokens = vec![
            Random(2),
            If(3), // out of range
            Title("A"),
            EndIf,
            EndRandom,
        ];
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
        assert_eq!(errors, vec![ControlFlowRule::RandomIfBranchValueOutOfRange]);
    }

    #[test]
    fn test_switch_duplicate_case() {
        use Token::*;
        let tokens = vec![
            Switch(2),
            Case(1),
            Title("A"),
            Case(1), // duplicate
            Title("B"),
            EndSwitch,
        ];
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
        assert_eq!(errors, vec![ControlFlowRule::SwitchDuplicateCaseValue]);
    }

    #[test]
    fn test_switch_case_value_out_of_range() {
        use Token::*;
        let tokens = vec![
            Switch(2),
            Case(3), // out of range
            Title("A"),
            EndSwitch,
        ];
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
        assert_eq!(errors, vec![ControlFlowRule::SwitchCaseValueOutOfRange]);
    }

    #[test]
    fn test_switch_duplicate_def() {
        use Token::*;
        let tokens = vec![
            Switch(2),
            Def,
            Title("A"),
            Def, // redundant
            Title("B"),
            Def, // redundant
            Title("C"),
            EndSwitch,
        ];
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
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
            Title("A"), // Not in any IfBlock
            If(1),
            Title("B"),
            EndIf,
            EndRandom,
        ];
        let mut errors = Vec::new();
        let _ = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
        assert_eq!(errors, vec![ControlFlowRule::UnmatchedTokenInRandomBlock]);
    }
}
