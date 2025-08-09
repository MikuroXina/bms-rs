//! The module for building the control flow AST.
//!
//! The AST is a tree structure that represents the control flow of the BMS source.
//! It is used to check the control flow of the BMS source.
//!
//! The AST is built by traversing the Token stream and recursively parsing all control flow blocks.
//!
//! The AST is used to check the control flow of the BMS source.

use std::collections::HashMap;

use num::BigUint;

use crate::bms::{
    BmsTokenIter, ast::structure::IfBlock, command::PositionWrapper, lex::token::TokenContent,
};
use crate::command::PositionWrapperExt;

use super::structure::{
    AstBuildWarning, AstBuildWarningType, BlockValue, CaseBranch, CaseBranchValue, IfBranch, Unit,
};

/// The main entry for building the control flow AST. Traverses the Token stream and recursively parses all control flow blocks.
/// Returns a list of AST nodes and collects all control flow related errors.
pub(super) fn build_control_flow_ast<'a>(
    tokens_iter: &mut BmsTokenIter<'a>,
) -> (Vec<Unit<'a>>, Vec<AstBuildWarning>) {
    let mut result = Vec::new();
    let mut errors = Vec::new();
    while tokens_iter.peek().is_some() {
        if let Some((unit, mut errs)) = parse_unit_or_block(tokens_iter) {
            result.push(unit);
            errors.append(&mut errs);
            continue;
        }
        let Some(token) = tokens_iter.peek() else {
            break;
        };
        use TokenContent::*;
        let rule = match &token.content {
            EndIf => Some(AstBuildWarningType::UnmatchedEndIf),
            EndRandom => Some(AstBuildWarningType::UnmatchedEndRandom),
            EndSwitch => Some(AstBuildWarningType::UnmatchedEndSwitch),
            ElseIf(_) => Some(AstBuildWarningType::UnmatchedElseIf),
            Else => Some(AstBuildWarningType::UnmatchedElse),
            Skip => Some(AstBuildWarningType::UnmatchedSkip),
            Case(_) => Some(AstBuildWarningType::UnmatchedCase),
            Def => Some(AstBuildWarningType::UnmatchedDef),
            _ => None,
        };
        if let Some(rule) = rule {
            errors.push(rule.into_wrapper(token));
        }
        // Jump to the next Token
        tokens_iter.next();
    }
    (result, errors)
}

/// Handle a single Token: if it is the start of a block, recursively call the block parser, otherwise return a Token node.
fn parse_unit_or_block<'a>(
    iter: &mut BmsTokenIter<'a>,
) -> Option<(Unit<'a>, Vec<AstBuildWarning>)> {
    let token = iter.peek()?;
    use TokenContent::*;
    match &token.content {
        SetSwitch(_) | Switch(_) => {
            let (unit, errs) = parse_switch_block(iter);
            Some((unit, errs))
        }
        Random(_) | SetRandom(_) => {
            let (unit, errs) = parse_random_block(iter);
            Some((unit, errs))
        }
        content if !content.is_control_flow_token() => {
            let t = *token;
            iter.next();
            Some((Unit::Token(t), Vec::new()))
        }
        _ => None,
    }
}

/// Parse a Switch/SetSwitch block until EndSwitch or auto-completion termination.
/// Supports Case/Def branches, error detection, and nested structures.
fn parse_switch_block<'a>(iter: &mut BmsTokenIter<'a>) -> (Unit<'a>, Vec<AstBuildWarning>) {
    let token = iter.next().unwrap();
    use TokenContent::*;
    let block_value = match &token.content {
        SetSwitch(val) => BlockValue::Set { value: val.clone() },
        Switch(val) => BlockValue::Random { max: val.clone() },
        _ => unreachable!(),
    };
    let mut cases = Vec::new();
    let mut seen_case_values = std::collections::HashSet::new();
    let mut seen_def = false;
    let mut errors = Vec::new();
    while let Some(next) = iter.peek() {
        match &next.content {
            Case(case_val) => {
                // Check for duplicates
                if seen_case_values.contains(case_val) {
                    errors.push(AstBuildWarningType::SwitchDuplicateCaseValue.into_wrapper(next));
                    iter.next();
                    let (_, mut errs) = parse_case_or_def_body(iter);
                    errors.append(&mut errs);
                    if let Some(PositionWrapper { content: Skip, .. }) = iter.peek() {
                        iter.next();
                    }
                    continue;
                }
                let row = next.row;
                let col = next.column;
                iter.next();
                seen_case_values.insert(case_val);
                let (tokens, mut errs) = parse_case_or_def_body(iter);
                errors.append(&mut errs);
                cases.push(CaseBranch {
                    value: CaseBranchValue::Case(case_val.clone()).into_wrapper_manual(row, col),
                    tokens,
                });
                if let Some(PositionWrapper { content: Skip, .. }) = iter.peek() {
                    iter.next();
                }
            }
            Def => {
                if seen_def {
                    errors.push(AstBuildWarningType::SwitchDuplicateDef.into_wrapper(next));
                    iter.next();
                    let (_, mut errs) = parse_case_or_def_body(iter);
                    errors.append(&mut errs);
                    if let Some(PositionWrapper { content: Skip, .. }) = iter.peek() {
                        iter.next();
                    }
                    continue;
                }
                seen_def = true;
                let row = next.row;
                let col = next.column;
                iter.next();
                let (tokens, mut errs) = parse_case_or_def_body(iter);
                errors.append(&mut errs);
                cases.push(CaseBranch {
                    value: CaseBranchValue::Def.into_wrapper_manual(row, col),
                    tokens,
                });
                if let Some(PositionWrapper { content: Skip, .. }) = iter.peek() {
                    iter.next();
                }
            }
            EndSwitch => {
                iter.next();
                break;
            }
            EndIf => {
                errors.push(AstBuildWarningType::UnmatchedEndIf.into_wrapper(next));
                iter.next();
            }
            EndRandom => {
                errors.push(AstBuildWarningType::UnmatchedEndRandom.into_wrapper(next));
                iter.next();
            }
            // Automatically complete EndSwitch: break when encountering Random/SetRandom/If/EndRandom/EndIf
            Random(_) | SetRandom(_) | If(_) => {
                break;
            }
            _ => {
                iter.next();
            }
        }
    }
    // If the iterator has ended, also break (i.e., automatically complete EndSwitch)
    (
        Unit::SwitchBlock {
            value: block_value,
            cases,
        },
        errors,
    )
}

/// Parse the body of a Case/Def branch until a branch-terminating Token is encountered.
/// Supports nested blocks, prioritizing parse_unit_or_block.
fn parse_case_or_def_body<'a>(
    iter: &mut BmsTokenIter<'a>,
) -> (Vec<Unit<'a>>, Vec<AstBuildWarning>) {
    let mut result = Vec::new();
    let mut errors = Vec::new();
    use TokenContent::*;
    while let Some(&token) = iter.peek() {
        if matches!(&token.content, Skip | EndSwitch | Case(_) | Def) {
            break;
        }
        if let Some((unit, mut errs)) = parse_unit_or_block(iter) {
            result.push(unit);
            errors.append(&mut errs);
            continue;
        }
        let rule = match &token.content {
            EndIf => Some(AstBuildWarningType::UnmatchedEndIf),
            EndRandom => Some(AstBuildWarningType::UnmatchedEndRandom),
            EndSwitch => Some(AstBuildWarningType::UnmatchedEndSwitch),
            ElseIf(_) => Some(AstBuildWarningType::UnmatchedElseIf),
            Else => Some(AstBuildWarningType::UnmatchedElse),
            Skip => Some(AstBuildWarningType::UnmatchedSkip),
            _ => None,
        };
        if let Some(rule) = rule {
            errors.push(rule.into_wrapper(token));
        }
        // Jump to the next Token
        iter.next();
    }
    (result, errors)
}

/// Parse a Random/SetRandom block until EndRandom or auto-completion termination.
/// Supports nesting, error detection, and records errors when non-control-flow Tokens appear outside IfBlock.
/// Design:
/// - After entering Random/SetRandom, loop through Tokens.
/// - If encountering If/ElseIf/Else, collect branches and check for duplicates/out-of-range.
/// - If encountering a non-control-flow Token, prioritize parse_unit_or_block; if not in any IfBlock, report error.
/// - Supports nested structures; recursively handle other block types.
fn parse_random_block<'a>(iter: &mut BmsTokenIter<'a>) -> (Unit<'a>, Vec<AstBuildWarning>) {
    // 1. Read the Random/SetRandom header to determine the max branch value
    let token = iter.next().unwrap();
    use TokenContent::*;
    let block_value = match &token.content {
        Random(val) => BlockValue::Random { max: val.clone() },
        SetRandom(val) => BlockValue::Set { value: val.clone() },
        _ => unreachable!(),
    };
    let mut if_blocks = Vec::new();
    let mut errors = Vec::new();
    // 2. Main loop, process the contents inside the Random block
    while let Some(PositionWrapper {
        content,
        row,
        column,
    }) = iter.peek()
    {
        match content {
            // 2.1 Handle If branch
            If(if_val) => {
                let (row, col) = {
                    let t = iter.peek().unwrap();
                    (t.row, t.column)
                };
                iter.next();
                let mut branches = HashMap::new();
                let mut seen_if_values = std::collections::HashSet::new();
                // Check if If branch value is duplicated
                if seen_if_values.contains(if_val) {
                    errors.push(
                        AstBuildWarningType::RandomDuplicateIfBranchValue
                            .into_wrapper_manual(row, col),
                    );
                    let (_, mut errs) = parse_if_block_body(iter);
                    errors.append(&mut errs);
                } else {
                    seen_if_values.insert(if_val);
                    let (tokens, mut errs) = parse_if_block_body(iter);
                    errors.append(&mut errs);
                    branches.insert(
                        if_val.clone(),
                        IfBranch {
                            value: if_val.clone().into_wrapper_manual(row, col),
                            tokens,
                        },
                    );
                }
                // 2.2 Handle ElseIf branches, same logic as If
                while let Some(PositionWrapper {
                    content: ElseIf(elif_val),
                    row,
                    column,
                }) = iter.peek()
                {
                    if seen_if_values.contains(elif_val) {
                        errors.push(
                            AstBuildWarningType::RandomDuplicateIfBranchValue
                                .into_wrapper_manual(*row, *column),
                        );
                        iter.next();
                        let (_, mut errs) = parse_if_block_body(iter);
                        errors.append(&mut errs);
                        continue;
                    }
                    iter.next();
                    seen_if_values.insert(elif_val);
                    let (elif_tokens, mut errs) = parse_if_block_body(iter);
                    errors.append(&mut errs);
                    branches.insert(
                        elif_val.clone(),
                        IfBranch {
                            value: elif_val.clone().into_wrapper_manual(*row, *column),
                            tokens: elif_tokens,
                        },
                    );
                }
                // 2.3 Check for redundant ElseIf
                if let Some(PositionWrapper {
                    content: ElseIf(_),
                    row,
                    column,
                }) = iter.peek()
                {
                    errors.push(
                        AstBuildWarningType::UnmatchedElseIf.into_wrapper_manual(*row, *column),
                    );
                    iter.next();
                }
                // 2.4 Handle Else branch, branch value is 0
                if let Some(PositionWrapper { content: Else, .. }) = iter.peek() {
                    let (row, col) = {
                        let t = iter.peek().unwrap();
                        (t.row, t.column)
                    };
                    iter.next();
                    let (etokens, mut errs) = parse_if_block_body(iter);
                    errors.append(&mut errs);
                    branches.insert(
                        BigUint::from(0u64),
                        IfBranch {
                            value: BigUint::from(0u64).into_wrapper_manual(row, col),
                            tokens: etokens,
                        },
                    );
                }
                // 2.5 Check for redundant Else
                if let Some(PositionWrapper { content: Else, .. }) = iter.peek() {
                    errors.push(AstBuildWarningType::UnmatchedElse.into_wrapper_manual(row, col));
                    iter.next();
                }
                // 2.6 Collect this IfBlock
                if_blocks.push(IfBlock { branches });
            }
            // 3.1 Termination: EndRandom encountered, block ends
            EndRandom => {
                iter.next();
                break;
            }
            // 3.2 Error: EndIf/EndSwitch encountered, record error and skip
            EndIf => {
                errors.push(AstBuildWarningType::UnmatchedEndIf.into_wrapper_manual(*row, *column));
                iter.next();
            }
            EndSwitch => {
                errors.push(
                    AstBuildWarningType::UnmatchedEndSwitch.into_wrapper_manual(*row, *column),
                );
                iter.next();
            }
            // 3.3 Auto-completion termination: break early when encountering other block headers or Case/Def/Skip
            SetSwitch(_) | Switch(_) | Case(_) | Def | Skip => {
                break;
            }
            // 4. Handle non-control-flow Token: prioritize parse_unit_or_block; if not in any IfBlock, report error
            _ => {
                if let Some((_unit, mut errs)) = parse_unit_or_block(iter) {
                    // This Token does not belong to any IfBlock, discard directly and record error
                    errors
                        .push(AstBuildWarningType::UnmatchedTokenInRandomBlock.into_wrapper(token));
                    errors.append(&mut errs);
                } else {
                    iter.next();
                }
            }
        }
    }
    // 5. Return AST node
    (
        Unit::RandomBlock {
            value: block_value,
            if_blocks,
        },
        errors,
    )
}

/// Parse the body of an If/ElseIf/Else branch until a branch-terminating Token is encountered.
/// Design:
/// - Supports nested blocks, prioritizing parse_unit_or_block.
/// - Break when encountering branch-terminating Tokens (ElseIf/Else/EndIf/EndRandom/EndSwitch).
/// - If EndIf is encountered, consume it automatically.
fn parse_if_block_body<'a>(iter: &mut BmsTokenIter<'a>) -> (Vec<Unit<'a>>, Vec<AstBuildWarning>) {
    let mut result = Vec::new();
    let mut errors = Vec::new();
    use TokenContent::*;
    while let Some(token) = iter.peek() {
        match &token.content {
            // 1. Branch-terminating Token, break
            ElseIf(_) | Else | EndIf | EndRandom | EndSwitch => {
                if let EndIf = token.content {
                    iter.next();
                }
                break;
            }
            // 2. Other content, prioritizing parse_unit_or_block (supports nesting)
            _ => {
                if let Some((unit, mut errs)) = parse_unit_or_block(iter) {
                    result.push(unit);
                    errors.append(&mut errs);
                } else {
                    iter.next();
                }
            }
        }
    }
    (result, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bms::ast::rng::Rng;
    use crate::bms::{
        ast::ast_parse::parse_control_flow_ast, command::PositionWrapper, lex::token::TokenContent,
    };
    use core::ops::RangeInclusive;
    use num::BigUint;

    #[test]
    fn test_switch_ast() {
        use TokenContent::*;
        let tokens = vec![
            SetSwitch(BigUint::from(2u64)),
            Def,
            Title("Out"),
            Case(BigUint::from(2u64)),
            Title("In 1"),
            Case(BigUint::from(1u64)),
            Title("In 2"),
            Skip,
            Case(BigUint::from(3u64)),
            Title("In 3"),
            Skip,
            EndSwitch,
        ]
        .into_iter()
        .map(|t| PositionWrapper::<TokenContent> {
            content: t,
            row: 0,
            column: 0,
        })
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut BmsTokenIter::from_tokens(&tokens));
        assert_eq!(errors, vec![]);
        let Some(Unit::SwitchBlock { cases, .. }) =
            ast.iter().find(|u| matches!(u, Unit::SwitchBlock { .. }))
        else {
            panic!("AST structure error, ast: {ast:?}");
        };
        let Some(_case1) = cases.iter().find(
            |c| matches!(*c.value, CaseBranchValue::Case(ref val) if val == &BigUint::from(1u64)),
        ) else {
            panic!("Case(1) not found, cases: {cases:?}");
        };
        let Some(Unit::SwitchBlock { cases, .. }) =
            ast.iter().find(|u| matches!(u, Unit::SwitchBlock { .. }))
        else {
            panic!("AST structure error, ast: {ast:?}");
        };
        let Some(CaseBranch { tokens: _, .. }) = cases.iter().find(
            |c| matches!(*c.value, CaseBranchValue::Case(ref val) if val == &BigUint::from(1u64)),
        ) else {
            panic!("Case(1) not found, cases: {cases:?}");
        };
        // Since tokens only contain Token type, do not search for RandomBlock here. Related assertions are already covered above.
    }

    #[test]
    fn test_unmatched_endrandom_error() {
        use TokenContent::*;
        let tokens = [Title("A"), EndRandom]
            .into_iter()
            .map(|t| PositionWrapper::<TokenContent> {
                content: t,
                row: 0,
                column: 0,
            })
            .collect::<Vec<_>>();
        let (_ast, errors) = build_control_flow_ast(&mut BmsTokenIter::from_tokens(&tokens));
        assert!(errors.contains(&AstBuildWarningType::UnmatchedEndRandom.into_wrapper(&tokens[1])));
    }

    #[test]
    fn test_unmatched_endif_error() {
        use TokenContent::*;
        let tokens = [Title("A"), EndIf]
            .into_iter()
            .map(|t| PositionWrapper::<TokenContent> {
                content: t,
                row: 0,
                column: 0,
            })
            .collect::<Vec<_>>();
        let (_ast, errors) = build_control_flow_ast(&mut BmsTokenIter::from_tokens(&tokens));
        assert!(errors.contains(&AstBuildWarningType::UnmatchedEndIf.into_wrapper(&tokens[1])));
    }

    #[test]
    fn test_random_ast() {
        use TokenContent::*;
        let tokens = vec![
            Random(BigUint::from(2u64)),
            If(BigUint::from(1u64)),
            Title("A"),
            EndIf,
            If(BigUint::from(2u64)),
            Title("B"),
            EndIf,
            EndRandom,
        ]
        .into_iter()
        .map(|t| PositionWrapper::<TokenContent> {
            content: t,
            row: 0,
            column: 0,
        })
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut BmsTokenIter::from_tokens(&tokens));
        assert_eq!(errors, vec![]);
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
        let Some(_) = all_titles.iter().find(|u| {
            matches!(
                u,
                Unit::Token(PositionWrapper::<TokenContent> {
                    content: Title("A"),
                    ..
                })
            )
        }) else {
            panic!("A missing, all_titles: {all_titles:?}");
        };
        let Some(_) = all_titles.iter().find(|u| {
            matches!(
                u,
                Unit::Token(PositionWrapper::<TokenContent> {
                    content: Title("B"),
                    ..
                })
            )
        }) else {
            panic!("B missing, all_titles: {all_titles:?}");
        };
    }

    #[test]
    fn test_random_nested_ast() {
        use TokenContent::*;
        let tokens = vec![
            Random(BigUint::from(2u64)),
            If(BigUint::from(1u64)),
            Title("A"),
            Random(BigUint::from(2u64)),
            If(BigUint::from(2u64)),
            Title("B"),
            EndIf,
            EndRandom,
            EndIf,
            EndRandom,
        ]
        .into_iter()
        .map(|t| PositionWrapper::<TokenContent> {
            content: t,
            row: 0,
            column: 0,
        })
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut BmsTokenIter::from_tokens(&tokens));
        assert_eq!(errors, vec![]);
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
        use TokenContent::*;
        let tokens = vec![
            Random(BigUint::from(3u64)),
            If(BigUint::from(1u64)),
            Title("A1"),
            ElseIf(BigUint::from(2u64)),
            Title("A2"),
            Else,
            Title("Aelse"),
            EndIf,
            If(BigUint::from(1u64)),
            Title("B1"),
            ElseIf(BigUint::from(2u64)),
            Title("B2"),
            Else,
            Title("Belse"),
            EndIf,
            EndRandom,
        ]
        .into_iter()
        .map(|t| PositionWrapper::<TokenContent> {
            content: t,
            row: 0,
            column: 0,
        })
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut BmsTokenIter::from_tokens(&tokens));
        assert_eq!(errors, vec![]);
        let Unit::RandomBlock {
            value: _,
            if_blocks,
        } = &ast[0]
        else {
            panic!("AST structure error");
        };
        assert_eq!(if_blocks.len(), 2);
        let branches1 = &if_blocks[0].branches;
        let Some(b1) = branches1.get(&BigUint::from(1u64)) else {
            panic!("branch 1 missing");
        };
        let Some(_) = b1.tokens.iter().find(|u| {
            matches!(
                u,
                Unit::Token(PositionWrapper::<TokenContent> {
                    content: Title("A1"),
                    ..
                })
            )
        }) else {
            panic!("A1 missing");
        };
        let Some(b2) = branches1.get(&BigUint::from(2u64)) else {
            panic!("branch 2 missing");
        };
        let Some(_) = b2.tokens.iter().find(|u| {
            matches!(
                u,
                Unit::Token(PositionWrapper::<TokenContent> {
                    content: Title("A2"),
                    ..
                })
            )
        }) else {
            panic!("A2 missing");
        };
        let Some(belse) = branches1.get(&BigUint::from(0u64)) else {
            panic!("branch else missing");
        };
        let Some(_) = belse.tokens.iter().find(|u| {
            matches!(
                u,
                Unit::Token(PositionWrapper::<TokenContent> {
                    content: Title("Aelse"),
                    ..
                })
            )
        }) else {
            panic!("Aelse missing");
        };
        let branches2 = &if_blocks[1].branches;
        let Some(b1) = branches2.get(&BigUint::from(1u64)) else {
            panic!("branch 1 missing");
        };
        let Some(_) = b1.tokens.iter().find(|u| {
            matches!(
                u,
                Unit::Token(PositionWrapper::<TokenContent> {
                    content: Title("B1"),
                    ..
                })
            )
        }) else {
            panic!("B1 missing");
        };
        let Some(b2) = branches2.get(&BigUint::from(2u64)) else {
            panic!("branch 2 missing");
        };
        let Some(_) = b2.tokens.iter().find(|u| {
            matches!(
                u,
                Unit::Token(PositionWrapper::<TokenContent> {
                    content: Title("B2"),
                    ..
                })
            )
        }) else {
            panic!("B2 missing");
        };
        let Some(belse) = branches2.get(&BigUint::from(0u64)) else {
            panic!("branch else missing");
        };
        let Some(_) = belse.tokens.iter().find(|u| {
            matches!(
                u,
                Unit::Token(PositionWrapper::<TokenContent> {
                    content: Title("Belse"),
                    ..
                })
            )
        }) else {
            panic!("Belse missing");
        };
    }

    #[test]
    fn test_random_duplicate_ifbranch() {
        use TokenContent::*;
        let tokens = vec![
            Random(BigUint::from(2u64)),
            If(BigUint::from(1u64)),
            Title("A"),
            ElseIf(BigUint::from(1u64)), // duplicate
            Title("B"),
            EndIf,
            EndRandom,
        ]
        .into_iter()
        .map(|t| PositionWrapper::<TokenContent> {
            content: t,
            row: 0,
            column: 0,
        })
        .collect::<Vec<_>>();
        let (_ast, errors) = build_control_flow_ast(&mut BmsTokenIter::from_tokens(&tokens));
        assert_eq!(
            errors,
            vec![AstBuildWarningType::RandomDuplicateIfBranchValue.into_wrapper(&tokens[3])]
        );
    }

    #[test]
    fn test_random_ifbranch_value_out_of_range() {
        use TokenContent::*;
        let tokens = vec![
            Random(BigUint::from(2u64)),
            If(BigUint::from(3u64)), // out of range
            Title("A"),
            EndIf,
            EndRandom,
        ]
        .into_iter()
        .map(|t| PositionWrapper::<TokenContent> {
            content: t,
            row: 0,
            column: 0,
        })
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut BmsTokenIter::from_tokens(&tokens));
        assert_eq!(errors, vec![]);
        // Now validate at parse stage
        struct DummyRng;
        impl Rng for DummyRng {
            fn generate(&mut self, range: RangeInclusive<BigUint>) -> BigUint {
                range.end().clone()
            }
        }
        let mut rng = DummyRng;
        let mut iter = ast.into_iter().peekable();
        let (_tokens, warnings) = parse_control_flow_ast(&mut iter, &mut rng);
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0].content,
            super::super::structure::AstParseWarningType::RandomIfBranchValueOutOfRange
        ));
    }

    #[test]
    fn test_switch_duplicate_case() {
        use TokenContent::*;
        let tokens = vec![
            Switch(BigUint::from(2u64)),
            Case(BigUint::from(1u64)),
            Title("A"),
            Case(BigUint::from(1u64)), // duplicate
            Title("B"),
            EndSwitch,
        ]
        .into_iter()
        .map(|t| PositionWrapper::<TokenContent> {
            content: t,
            row: 0,
            column: 0,
        })
        .collect::<Vec<_>>();
        let (_ast, errors) = build_control_flow_ast(&mut BmsTokenIter::from_tokens(&tokens));
        assert_eq!(
            errors,
            vec![AstBuildWarningType::SwitchDuplicateCaseValue.into_wrapper(&tokens[3])]
        );
    }

    #[test]
    fn test_switch_case_value_out_of_range() {
        use TokenContent::*;
        let tokens = vec![
            Switch(BigUint::from(2u64)),
            Case(BigUint::from(3u64)), // out of range
            Title("A"),
            EndSwitch,
        ]
        .into_iter()
        .map(|t| PositionWrapper::<TokenContent> {
            content: t,
            row: 0,
            column: 0,
        })
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut BmsTokenIter::from_tokens(&tokens));
        assert_eq!(errors, vec![]);
        // Now validate at parse stage
        struct DummyRng;
        impl Rng for DummyRng {
            fn generate(&mut self, range: RangeInclusive<BigUint>) -> BigUint {
                range.end().clone()
            }
        }
        let mut rng = DummyRng;
        let mut iter = ast.into_iter().peekable();
        let (_tokens, warnings) = parse_control_flow_ast(&mut iter, &mut rng);
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0].content,
            super::super::structure::AstParseWarningType::SwitchCaseValueOutOfRange
        ));
    }

    #[test]
    fn test_switch_duplicate_def() {
        use TokenContent::*;
        let tokens = vec![
            Switch(BigUint::from(2u64)),
            Def,
            Title("A"),
            Def, // redundant
            Title("B"),
            Def, // redundant
            Title("C"),
            EndSwitch,
        ]
        .into_iter()
        .map(|t| PositionWrapper::<TokenContent> {
            content: t,
            row: 0,
            column: 0,
        })
        .collect::<Vec<_>>();
        let (_ast, errors) = build_control_flow_ast(&mut BmsTokenIter::from_tokens(&tokens));
        assert_eq!(
            errors,
            vec![
                AstBuildWarningType::SwitchDuplicateDef.into_wrapper(&tokens[2]),
                AstBuildWarningType::SwitchDuplicateDef.into_wrapper(&tokens[4]),
            ]
        );
    }

    #[test]
    fn test_unmatched_token_in_random_block() {
        use TokenContent::*;
        let tokens = vec![
            Random(BigUint::from(2u64)),
            Title("A"), // Not in any IfBlock
            If(BigUint::from(1u64)),
            Title("B"),
            EndIf,
            EndRandom,
        ]
        .into_iter()
        .map(|t| PositionWrapper::<TokenContent> {
            content: t,
            row: 0,
            column: 0,
        })
        .collect::<Vec<_>>();
        let (_ast, errors) = build_control_flow_ast(&mut BmsTokenIter::from_tokens(&tokens));
        assert_eq!(
            errors,
            vec![AstBuildWarningType::UnmatchedTokenInRandomBlock.into_wrapper(&tokens[1])]
        );
    }
}
