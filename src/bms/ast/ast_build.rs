use std::{collections::HashMap, iter::Peekable};

use num::BigUint;

use crate::bms::{
    ast::{
        AstBuildWarningWithPos,
        structure::{BlockValue, CaseBranch, CaseBranchValue, IfBlock, IfBranch, Unit},
    },
    command::mixin::SourcePosMixinExt,
    lex::token::{Token, TokenWithPos},
};

use super::AstBuildWarning;

/// The main entry for building the control flow AST. Traverses the TokenWithPos stream and recursively parses all control flow blocks.
/// Returns a list of AST nodes and collects all control flow related errors.
pub(super) fn build_control_flow_ast<'a, T: Iterator<Item = &'a TokenWithPos<'a>>>(
    tokens_iter: &mut Peekable<T>,
) -> (Vec<Unit<'a>>, Vec<AstBuildWarningWithPos>) {
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
        use Token::*;
        let rule = match token.content() {
            EndIf => Some(AstBuildWarning::UnmatchedEndIf),
            EndRandom => Some(AstBuildWarning::UnmatchedEndRandom),
            EndSwitch => Some(AstBuildWarning::UnmatchedEndSwitch),
            ElseIf(_) => Some(AstBuildWarning::UnmatchedElseIf),
            Else => Some(AstBuildWarning::UnmatchedElse),
            Skip => Some(AstBuildWarning::UnmatchedSkip),
            Case(_) => Some(AstBuildWarning::UnmatchedCase),
            Def => Some(AstBuildWarning::UnmatchedDef),
            _ => None,
        };
        if let Some(rule) = rule {
            errors.push(rule.into_wrapper(token));
        }
        // Jump to the next TokenWithPos
        tokens_iter.next();
    }
    (result, errors)
}

/// Handle a single TokenWithPos: if it is the start of a block, recursively call the block parser, otherwise return a TokenWithPos node.
fn parse_unit_or_block<'a, T: Iterator<Item = &'a TokenWithPos<'a>>>(
    iter: &mut Peekable<T>,
) -> Option<(Unit<'a>, Vec<AstBuildWarningWithPos>)> {
    let token = iter.peek()?;
    use Token::*;
    match token.content() {
        SetSwitch(_) | Switch(_) => {
            let (unit, errs) = parse_switch_block(iter);
            Some((unit, errs))
        }
        Random(_) | SetRandom(_) => {
            let (unit, errs) = parse_random_block(iter);
            Some((unit, errs))
        }
        content if !content.is_control_flow_token() => {
            let unit = Unit::TokenWithPos(token);
            iter.next();
            Some((unit, Vec::new()))
        }
        _ => None,
    }
}

/// Parse a Switch/SetSwitch block until EndSwitch or auto-completion termination.
/// Supports Case/Def branches, error detection, and nested structures.
fn parse_switch_block<'a, T: Iterator<Item = &'a TokenWithPos<'a>>>(
    iter: &mut Peekable<T>,
) -> (Unit<'a>, Vec<AstBuildWarningWithPos>) {
    let token = iter.next().unwrap();
    use Token::*;
    let block_value = match token.content() {
        SetSwitch(val) => BlockValue::Set { value: val.clone() },
        Switch(val) => BlockValue::Random { max: val.clone() },
        _ => unreachable!(),
    };
    let mut cases = Vec::new();
    let mut seen_case_values = std::collections::HashSet::new();
    let max_value = match &block_value {
        BlockValue::Random { max } => Some(max.clone()),
        BlockValue::Set { value: _ } => None,
    };
    let mut seen_def = false;
    let mut errors = Vec::new();
    while let Some(next) = iter.peek() {
        match next.content() {
            Case(case_val) => {
                // Check for duplicates
                if seen_case_values.contains(case_val) {
                    errors.push(AstBuildWarning::SwitchDuplicateCaseValue.into_wrapper(next));
                    iter.next();
                    let (_, mut errs) = parse_case_or_def_body(iter);
                    errors.append(&mut errs);
                    if iter
                        .peek()
                        .filter(|t| matches!(t.content(), Skip))
                        .is_some()
                    {
                        iter.next();
                    }
                    continue;
                }
                // Check for out-of-range
                if let Some(ref max) = max_value
                    && !(&BigUint::from(1u64)..=max).contains(&case_val)
                {
                    errors.push(AstBuildWarning::SwitchCaseValueOutOfRange.into_wrapper(next));
                    iter.next();
                    let (_, mut errs) = parse_case_or_def_body(iter);
                    errors.append(&mut errs);
                    if iter
                        .peek()
                        .filter(|t| matches!(t.content(), Skip))
                        .is_some()
                    {
                        iter.next();
                    }
                    continue;
                }
                iter.next();
                seen_case_values.insert(case_val);
                let (tokens, mut errs) = parse_case_or_def_body(iter);
                errors.append(&mut errs);
                cases.push(CaseBranch {
                    value: CaseBranchValue::Case(case_val.clone()),
                    units: tokens,
                });
                if iter
                    .peek()
                    .filter(|t| matches!(t.content(), Skip))
                    .is_some()
                {
                    iter.next();
                }
            }
            Def => {
                if seen_def {
                    errors.push(AstBuildWarning::SwitchDuplicateDef.into_wrapper(next));
                    iter.next();
                    let (_, mut errs) = parse_case_or_def_body(iter);
                    errors.append(&mut errs);
                    if iter
                        .peek()
                        .filter(|t| matches!(t.content(), Skip))
                        .is_some()
                    {
                        iter.next();
                    }
                    continue;
                }
                seen_def = true;
                iter.next();
                let (tokens, mut errs) = parse_case_or_def_body(iter);
                errors.append(&mut errs);
                cases.push(CaseBranch {
                    value: CaseBranchValue::Def,
                    units: tokens,
                });
                if iter
                    .peek()
                    .filter(|t| matches!(t.content(), Skip))
                    .is_some()
                {
                    iter.next();
                }
            }
            EndSwitch => {
                iter.next();
                break;
            }
            EndIf => {
                errors.push(AstBuildWarning::UnmatchedEndIf.into_wrapper(next));
                iter.next();
            }
            EndRandom => {
                errors.push(AstBuildWarning::UnmatchedEndRandom.into_wrapper(next));
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

/// Parse the body of a Case/Def branch until a branch-terminating TokenWithPos is encountered.
/// Supports nested blocks, prioritizing parse_unit_or_block.
fn parse_case_or_def_body<'a, T: Iterator<Item = &'a TokenWithPos<'a>>>(
    iter: &mut Peekable<T>,
) -> (Vec<Unit<'a>>, Vec<AstBuildWarningWithPos>) {
    let mut result = Vec::new();
    let mut errors = Vec::new();
    use Token::*;
    while let Some(&token) = iter.peek() {
        if matches!(token.content(), Skip | EndSwitch | Case(_) | Def) {
            break;
        }
        if let Some((unit, mut errs)) = parse_unit_or_block(iter) {
            result.push(unit);
            errors.append(&mut errs);
            continue;
        }
        let rule = match token.content() {
            EndIf => Some(AstBuildWarning::UnmatchedEndIf),
            EndRandom => Some(AstBuildWarning::UnmatchedEndRandom),
            EndSwitch => Some(AstBuildWarning::UnmatchedEndSwitch),
            ElseIf(_) => Some(AstBuildWarning::UnmatchedElseIf),
            Else => Some(AstBuildWarning::UnmatchedElse),
            Skip => Some(AstBuildWarning::UnmatchedSkip),
            _ => None,
        };
        if let Some(rule) = rule {
            errors.push(rule.into_wrapper(token));
        }
        // Jump to the next TokenWithPos
        iter.next();
    }
    (result, errors)
}

/// Parse a Random/SetRandom block until EndRandom or auto-completion termination.
/// Supports nesting, error detection, and auto-closes when encountering non-control-flow Tokens outside IfBlock.
/// Design:
/// - After entering Random/SetRandom, loop through Tokens.
/// - If encountering If/ElseIf/Else, collect branches and check for duplicates/out-of-range.
/// - If encountering a non-control-flow TokenWithPos, prioritize parse_unit_or_block; if not in any IfBlock, auto-close the block.
/// - Supports nested structures; recursively handle other block types.
fn parse_random_block<'a, T: Iterator<Item = &'a TokenWithPos<'a>>>(
    iter: &mut Peekable<T>,
) -> (Unit<'a>, Vec<AstBuildWarningWithPos>) {
    // 1. Read the Random/SetRandom header to determine the max branch value
    let token = iter.next().unwrap();
    use Token::*;
    let block_value = match token.content() {
        Random(val) => BlockValue::Random { max: val.clone() },
        SetRandom(val) => BlockValue::Set { value: val.clone() },
        _ => unreachable!(),
    };
    let mut if_blocks = Vec::new();
    let max_value = match &block_value {
        BlockValue::Random { max } => Some(max.clone()),
        BlockValue::Set { .. } => None,
    };
    let mut errors = Vec::new();
    // 2. Main loop, process the contents inside the Random block
    while let Some(&token) = iter.peek() {
        match token.content() {
            // 2.1 Handle If branch
            If(if_val) => {
                iter.next();
                let mut branches = HashMap::new();
                let mut seen_if_values = std::collections::HashSet::new();
                // Check if If branch value is duplicated
                if seen_if_values.contains(if_val) {
                    errors.push(
                        AstBuildWarning::RandomDuplicateIfBranchValue
                            .into_wrapper_manual(token.row(), token.column()),
                    );
                    let (_, mut errs) = parse_if_block_body(iter);
                    errors.append(&mut errs);
                } else if let Some(ref max) = max_value {
                    // Check if If branch value is out-of-range
                    if !(&BigUint::from(1u64)..=max).contains(&if_val) {
                        errors.push(
                            AstBuildWarning::RandomIfBranchValueOutOfRange
                                .into_wrapper_manual(token.row(), token.column()),
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
                                value: if_val.clone(),
                                units: tokens,
                            },
                        );
                    }
                } else {
                    // SetRandom branch has no range limit
                    seen_if_values.insert(if_val);
                    let (tokens, mut errs) = parse_if_block_body(iter);
                    errors.append(&mut errs);
                    branches.insert(
                        if_val.clone(),
                        IfBranch {
                            value: if_val.clone(),
                            units: tokens,
                        },
                    );
                }
                // 2.2 Handle ElseIf branches, same logic as If
                while let Some((token, elif_val)) = iter
                    .peek()
                    .map(|t| (t, t.content()))
                    .into_iter()
                    .filter_map(|(t, c)| match c {
                        ElseIf(val) => Some((t, val)),
                        _ => None,
                    })
                    .next()
                {
                    if seen_if_values.contains(elif_val) {
                        errors.push(
                            AstBuildWarning::RandomDuplicateIfBranchValue
                                .into_wrapper_manual(token.row(), token.column()),
                        );
                        iter.next();
                        let (_, mut errs) = parse_if_block_body(iter);
                        errors.append(&mut errs);
                        continue;
                    }
                    if let Some(ref max) = max_value
                        && !(&BigUint::from(1u64)..=max).contains(&elif_val)
                    {
                        errors.push(
                            AstBuildWarning::RandomIfBranchValueOutOfRange
                                .into_wrapper_manual(token.row(), token.column()),
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
                            value: elif_val.clone(),
                            units: elif_tokens,
                        },
                    );
                }
                // 2.3 Check for redundant ElseIf
                if let Some(token) = iter.peek().filter(|t| matches!(t.content(), ElseIf(_))) {
                    errors.push(
                        AstBuildWarning::UnmatchedElseIf
                            .into_wrapper_manual(token.row(), token.column()),
                    );
                    iter.next();
                }
                // 2.4 Handle Else branch, branch value is 0
                if let Some(_token) = iter.peek().filter(|t| matches!(t.content(), Else)) {
                    iter.next();
                    let (etokens, mut errs) = parse_if_block_body(iter);
                    errors.append(&mut errs);
                    branches.insert(
                        BigUint::from(0u64),
                        IfBranch {
                            value: BigUint::from(0u64),
                            units: etokens,
                        },
                    );
                }
                // 2.5 Check for redundant Else
                if let Some(token) = iter.peek().filter(|t| matches!(t.content(), Else)) {
                    errors.push(
                        AstBuildWarning::UnmatchedElse
                            .into_wrapper_manual(token.row(), token.column()),
                    );
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
                errors.push(
                    AstBuildWarning::UnmatchedEndIf
                        .into_wrapper_manual(token.row(), token.column()),
                );
                iter.next();
            }
            EndSwitch => {
                errors.push(
                    AstBuildWarning::UnmatchedEndSwitch
                        .into_wrapper_manual(token.row(), token.column()),
                );
                iter.next();
            }
            // 3.3 Auto-completion termination: break early when encountering other block headers or Case/Def/Skip
            SetSwitch(_) | Switch(_) | Case(_) | Def | Skip => {
                break;
            }
            // 4. Handle non-control-flow TokenWithPos: auto-close Random block when encountering non-control-flow commands
            content if !content.is_control_flow_token() => break,
            _ => {
                iter.next();
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

/// Parse the body of an If/ElseIf/Else branch until a branch-terminating TokenWithPos is encountered.
/// Design:
/// - Supports nested blocks, prioritizing parse_unit_or_block.
/// - Break when encountering branch-terminating Tokens (ElseIf/Else/EndIf/EndRandom/EndSwitch).
/// - If EndIf is encountered, consume it automatically.
fn parse_if_block_body<'a, T: Iterator<Item = &'a TokenWithPos<'a>>>(
    iter: &mut Peekable<T>,
) -> (Vec<Unit<'a>>, Vec<AstBuildWarningWithPos>) {
    let mut result = Vec::new();
    let mut errors = Vec::new();
    use Token::*;
    while let Some(token) = iter.peek() {
        match token.content() {
            // 1. Branch-terminating TokenWithPos, break
            ElseIf(_) | Else | EndIf | EndRandom | EndSwitch => {
                if let EndIf = token.content() {
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
    use crate::command::mixin::SourcePosMixinExt;

    #[test]
    fn test_switch_ast() {
        use Token::*;
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
        .enumerate()
        .map(|(i, t)| t.into_wrapper_manual(i, i))
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
        assert_eq!(errors, vec![]);
        let Some(Unit::SwitchBlock { cases, .. }) =
            ast.iter().find(|u| matches!(u, Unit::SwitchBlock { .. }))
        else {
            panic!("AST structure error, ast: {ast:?}");
        };
        let Some(_case1) = cases.iter().find(
            |c| matches!(c.value, CaseBranchValue::Case(ref val) if val == &BigUint::from(1u64)),
        ) else {
            panic!("Case(1) not found, cases: {cases:?}");
        };
        let Some(Unit::SwitchBlock { cases, .. }) =
            ast.iter().find(|u| matches!(u, Unit::SwitchBlock { .. }))
        else {
            panic!("AST structure error, ast: {ast:?}");
        };
        let Some(CaseBranch { units: _, .. }) = cases.iter().find(
            |c| matches!(c.value, CaseBranchValue::Case(ref val) if val == &BigUint::from(1u64)),
        ) else {
            panic!("Case(1) not found, cases: {cases:?}");
        };
        // Since tokens only contain TokenWithPos type, do not search for RandomBlock here. Related assertions are already covered above.
    }

    #[test]
    fn test_unmatched_endrandom_error() {
        use Token::*;
        let tokens = [Title("A"), EndRandom]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();
        let (_ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
        assert!(errors.contains(&AstBuildWarning::UnmatchedEndRandom.into_wrapper(&tokens[1])));
    }

    #[test]
    fn test_unmatched_endif_error() {
        use Token::*;
        let tokens = [Title("A"), EndIf]
            .into_iter()
            .enumerate()
            .map(|(i, t)| t.into_wrapper_manual(i, i))
            .collect::<Vec<_>>();
        let (_ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
        assert!(errors.contains(&AstBuildWarning::UnmatchedEndIf.into_wrapper(&tokens[1])));
    }

    #[test]
    fn test_random_ast() {
        use Token::*;
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
        .enumerate()
        .map(|(i, t)| t.into_wrapper_manual(i, i))
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
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
            .flat_map(|b| &b.units)
            .collect();
        let Some(_) = all_titles.iter().find(|u| {
            let Unit::TokenWithPos(token) = u else {
                panic!("Unit::TokenWithPos expected, got: {u:?}");
            };
            matches!(token.content(), Title("A"))
        }) else {
            panic!("A missing, all_titles: {all_titles:?}");
        };
        let Some(_) = all_titles.iter().find(|u| {
            let Unit::TokenWithPos(token) = u else {
                panic!("Unit::TokenWithPos expected, got: {u:?}");
            };
            matches!(token.content(), Title("B"))
        }) else {
            panic!("B missing, all_titles: {all_titles:?}");
        };
    }

    #[test]
    fn test_random_nested_ast() {
        use Token::*;
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
        .enumerate()
        .map(|(i, t)| t.into_wrapper_manual(i, i))
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
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
                    .units
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
        .enumerate()
        .map(|(i, t)| t.into_wrapper_manual(i, i))
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
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
        let Some(_) = b1.units.iter().find(|u| {
            let Unit::TokenWithPos(token) = u else {
                panic!("Unit::TokenWithPos expected, got: {u:?}");
            };
            matches!(token.content(), Title("A1"))
        }) else {
            panic!("A1 missing");
        };
        let Some(b2) = branches1.get(&BigUint::from(2u64)) else {
            panic!("branch 2 missing");
        };
        let Some(_) = b2.units.iter().find(|u| {
            let Unit::TokenWithPos(token) = u else {
                panic!("Unit::TokenWithPos expected, got: {u:?}");
            };
            matches!(token.content(), Title("A2"))
        }) else {
            panic!("A2 missing");
        };
        let Some(belse) = branches1.get(&BigUint::from(0u64)) else {
            panic!("branch else missing");
        };
        let Some(_) = belse.units.iter().find(|u| {
            let Unit::TokenWithPos(token) = u else {
                panic!("Unit::TokenWithPos expected, got: {u:?}");
            };
            matches!(token.content(), Title("Aelse"))
        }) else {
            panic!("Aelse missing");
        };
        let branches2 = &if_blocks[1].branches;
        let Some(b1) = branches2.get(&BigUint::from(1u64)) else {
            panic!("branch 1 missing");
        };
        let Some(_) = b1.units.iter().find(|u| {
            let Unit::TokenWithPos(token) = u else {
                panic!("Unit::TokenWithPos expected, got: {u:?}");
            };
            matches!(token.content(), Title("B1"))
        }) else {
            panic!("B1 missing");
        };
        let Some(b2) = branches2.get(&BigUint::from(2u64)) else {
            panic!("branch 2 missing");
        };
        let Some(_) = b2.units.iter().find(|u| {
            let Unit::TokenWithPos(token) = u else {
                panic!("Unit::TokenWithPos expected, got: {u:?}");
            };
            matches!(token.content(), Title("B2"))
        }) else {
            panic!("B2 missing");
        };
        let Some(belse) = branches2.get(&BigUint::from(0u64)) else {
            panic!("branch else missing");
        };
        let Some(_) = belse.units.iter().find(|u| {
            let Unit::TokenWithPos(token) = u else {
                panic!("Unit::TokenWithPos expected, got: {u:?}");
            };
            matches!(token.content(), Title("Belse"))
        }) else {
            panic!("Belse missing");
        };
    }

    #[test]
    fn test_random_duplicate_ifbranch() {
        use Token::*;
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
        .enumerate()
        .map(|(i, t)| t.into_wrapper_manual(i, i))
        .collect::<Vec<_>>();
        let (_ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
        assert_eq!(
            errors,
            vec![AstBuildWarning::RandomDuplicateIfBranchValue.into_wrapper(&tokens[3])]
        );
    }

    #[test]
    fn test_random_ifbranch_value_out_of_range() {
        use Token::*;
        let tokens = vec![
            Random(BigUint::from(2u64)),
            If(BigUint::from(3u64)), // out of range
            Title("A"),
            EndIf,
            EndRandom,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_manual(i, i))
        .collect::<Vec<_>>();
        let (_ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
        assert_eq!(
            errors,
            vec![AstBuildWarning::RandomIfBranchValueOutOfRange.into_wrapper(&tokens[1])]
        );
    }

    #[test]
    fn test_switch_duplicate_case() {
        use Token::*;
        let tokens = vec![
            Switch(BigUint::from(2u64)),
            Case(BigUint::from(1u64)),
            Title("A"),
            Case(BigUint::from(1u64)), // duplicate
            Title("B"),
            EndSwitch,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_manual(i, i))
        .collect::<Vec<_>>();
        let (_ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
        assert_eq!(
            errors,
            vec![AstBuildWarning::SwitchDuplicateCaseValue.into_wrapper(&tokens[3])]
        );
    }

    #[test]
    fn test_switch_case_value_out_of_range() {
        use Token::*;
        let tokens = vec![
            Switch(BigUint::from(2u64)),
            Case(BigUint::from(3u64)), // out of range
            Title("A"),
            EndSwitch,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_manual(i, i))
        .collect::<Vec<_>>();
        let (_ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
        assert_eq!(
            errors,
            vec![AstBuildWarning::SwitchCaseValueOutOfRange.into_wrapper(&tokens[1])]
        );
    }

    #[test]
    fn test_switch_duplicate_def() {
        use Token::*;
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
        .enumerate()
        .map(|(i, t)| t.into_wrapper_manual(i, i))
        .collect::<Vec<_>>();
        let (_ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
        assert_eq!(
            errors,
            vec![
                AstBuildWarning::SwitchDuplicateDef.into_wrapper(&tokens[3]),
                AstBuildWarning::SwitchDuplicateDef.into_wrapper(&tokens[5]),
            ]
        );
    }

    #[test]
    fn test_auto_close_random_block_on_non_control_flow() {
        use Token::*;
        let tokens = vec![
            Random(BigUint::from(2u64)),
            Title("A"), // Not in any IfBlock, should auto-close Random block
            Title("B"), // This should be outside the Random block
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_manual(i, i))
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
        // Should not produce any errors, Random block should auto-close
        assert_eq!(errors, vec![]);
        // Should have three units: RandomBlock and two TokenWithPos
        assert_eq!(ast.len(), 3);
        // First unit should be RandomBlock
        let Unit::RandomBlock { if_blocks, .. } = &ast[0] else {
            panic!("First unit should be RandomBlock, got: {:?}", ast[0]);
        };
        // RandomBlock should be empty (no If blocks)
        assert_eq!(if_blocks.len(), 0);
        // Second unit should be the first Title token
        let Unit::TokenWithPos(token) = &ast[1] else {
            panic!("Second unit should be TokenWithPos, got: {:?}", ast[1]);
        };
        assert!(matches!(token.content(), Title("A")));
        // Third unit should be the second Title token
        let Unit::TokenWithPos(token) = &ast[2] else {
            panic!("Third unit should be TokenWithPos, got: {:?}", ast[2]);
        };
        assert!(matches!(token.content(), Title("B")));
    }

    #[test]
    fn test_auto_close_random_block_with_multiple_if_blocks() {
        use Token::*;
        // This simulates the scenario from the reference:
        // #RANDOM 2
        // #IF 1
        // #00112:00220000
        // #ENDIF
        // #IF 2
        // #00113:00003300
        // #ENDIF
        // #00114:00000044  <- This should auto-close the Random block
        let tokens = vec![
            Random(BigUint::from(2u64)),
            If(BigUint::from(1u64)),
            Title("00112:00220000"),
            EndIf,
            If(BigUint::from(2u64)),
            Title("00113:00003300"),
            EndIf,
            Title("00114:00000044"), // This should auto-close the Random block
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_manual(i, i))
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
        // Should not produce any errors
        assert_eq!(errors, vec![]);
        // Should have two units: RandomBlock and TokenWithPos
        assert_eq!(ast.len(), 2);
        // First unit should be RandomBlock with two If blocks
        let Unit::RandomBlock { if_blocks, .. } = &ast[0] else {
            panic!("First unit should be RandomBlock, got: {:?}", ast[0]);
        };
        assert_eq!(if_blocks.len(), 2);
        // Second unit should be the final Title token
        let Unit::TokenWithPos(token) = &ast[1] else {
            panic!("Second unit should be TokenWithPos, got: {:?}", ast[1]);
        };
        assert!(matches!(token.content(), Title("00114:00000044")));
    }
}
