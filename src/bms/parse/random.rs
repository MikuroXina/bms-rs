use std::ops::ControlFlow::{self, *};

use thiserror::Error;

use super::{rng::Rng, ParseError, Result};
use crate::lex::token::Token;

/// An error occurred when parsing the [`TokenStream`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum ControlFlowRule {
    #[error(
        "Other command should be in a #RANDOM(#SWITCH) and its #IF/#ELSE/#ELSE(#CASE/#DEF) block"
    )]
    CommandInRandomBlockAndIfBlock,
    #[error("#RANDOM block and #SWITCH block commands should not mixed.")]
    RandomAndSwitchCommandNotMix,
    #[error("#RANDOM/#SETRANDOM(#SWITCH/#SETSWITCH) command must come in root of #IF/#ELSEIF/#ELSE(#CASE/#DEF) block")]
    RandomsInRootOrIfsBlock,
    #[error("#IF/#ELSEIF/#ELSE/#ENDIF command must be in #RANDOM - #ENDRANDOM block")]
    IfsInRandomBlock,
    #[error("#CASE/#SKIP/#DEF command must be in #SWITCH - #ENDSW block")]
    CasesInSwitchBlock,
    #[error("Only 1 #IF command is allowed in a #RANDOM - #ENDRANDOM block")]
    OnlyOneIfInRandomBlock,
    #[error("Only 1 #ELSE command is allowed in a #RANDOM - #ENDRANDOM block")]
    OnlyOneElseInRandomBlock,
    #[error("Only 1 #DEF command is allowed in a #SWITCH - #ENDSW block")]
    OnlyOneDefaultInSwitchBlock,
    #[error("#ELSEIF command must come after #IF block")]
    ElseIfAfterIf,
    #[error("#ELSE command must come after #IF/#ELESIF block")]
    ElseAfterIfs,
    #[error("#ENDIF command must come after #IF/#ELSEIF/#ELSE block")]
    EndIfAfterIfs,
    #[error("#ENDRANDOM command must come after #RANDOM block")]
    EndRandomAfterRandomBlock,
    #[error("#ENDRANDOM command must come after #ENDIF")]
    EndRandomAfterEndif,
    #[error("#ENDSW command must come after #SWITCH block")]
    EndSwitchAfterSwitchBlock,
    #[error("#DEF command must come after #CASE")]
    DefaultAfterCase,
    #[error("#IF/#ELSEIF(#CASE) command's value is out of the range of [1, max]")]
    ValueOutOfRange,
    #[error("Values in #IF/#ELSEIF/#ELSE group should be unique")]
    ValuesInIfGroupShouldBeUnique,
}

impl From<ControlFlowRule> for ParseError {
    fn from(rule: ControlFlowRule) -> Self {
        ParseError::ViolateControlFlowRule(rule)
    }
}

impl<T> From<ControlFlowRule> for Result<T> {
    fn from(rule: ControlFlowRule) -> Self {
        Err(ParseError::from(rule))
    }
}

impl<T> From<ControlFlowRule> for ControlFlow<Result<T>> {
    fn from(rule: ControlFlowRule) -> Self {
        Break(<std::result::Result<T, ParseError>>::from(rule))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ControlFlowBlock {
    Random {
        /// It can be used for warning unreachable branch
        rand_max: u32,
        /// If the parent part cannot pass, this will be None,
        chosen_value: Option<u32>,
    },
    IfBranch {
        /// It can be used for warning unreachable `#ELESIF`
        /// If the parent part cannot pass, this will be None,
        matching_value: u32,
        /// If there is any #IF/#ELSEIF branch has been matched previously in this #IF - (#ELSEIF/#ELSE) -
        /// #ENDIF block list (called #IF group). Used by #ELSE.
        /// One #RANDOM - #ENDRANDOM block can contain more than 1 #IF group.
        group_previously_matched: bool,
    },
    ElseBranch {
        /// Passed in this else
        else_activate: bool,
    },
    Switch {
        /// It can be used for warning unreachable branch
        switch_max: u32,
        /// If the parent part cannot pass, this will be None,
        chosen_value: Option<u32>,
        /// Whether skipping tokens until `#ENDSW`
        skipping: bool,
    },
    CaseBranch {
        /// It can be used for warning unreachable `#CASE`
        matching_value: u32,
    },
    DefaultBranch,
}

impl ControlFlowBlock {
    pub fn is_type_random(&self) -> bool {
        use ControlFlowBlock::*;
        matches!(self, Random { .. } | IfBranch { .. } | ElseBranch { .. })
    }
    pub fn is_type_switch(&self) -> bool {
        use ControlFlowBlock::*;
        matches!(self, Switch { .. } | CaseBranch { .. } | DefaultBranch)
    }
}

pub struct RandomParser<R> {
    rng: R,
    stack: Vec<ControlFlowBlock>,
}

impl<R: Rng> RandomParser<R> {
    pub fn new(rng: R) -> Self {
        Self { rng, stack: vec![] }
    }

    /// Because in the parse(), if the last floor not match, this Random/Switch's chosen_value will
    /// be none. So just check this floor.
    fn is_all_match(&self) -> bool {
        use ControlFlowBlock::*;
        let Some((root_index, root)) = self
            .stack
            .iter()
            .enumerate()
            .rfind(|(_i, block)| matches!(block, Random { .. } | Switch { .. }))
        else {
            return true;
        };
        let (_former_part, latter_part) = self.stack.split_at(root_index + 1);
        match root {
            Random {
                chosen_value: Some(chosen_value),
                ..
            } => latter_part.iter().any(|block| match block {
                IfBranch {
                    matching_value,
                    group_previously_matched,
                } => !group_previously_matched && matching_value == chosen_value,
                ElseBranch { else_activate } => *else_activate,
                _ => false,
            }),
            Switch {
                chosen_value: Some(chosen_value),
                skipping,
                ..
            } => latter_part.iter().any(|block| match block {
                CaseBranch { matching_value } => !skipping && matching_value == chosen_value,
                DefaultBranch => !skipping,
                _ => false,
            }),
            _ => false,
        }
    }

    pub fn parse(&mut self, token: &Token) -> ControlFlow<Result<()>> {
        use ControlFlowBlock::*;
        use ControlFlowRule::*;
        match *token {
            // Part: Random
            Token::Random(rand_max) => {
                if matches!(self.stack.last(), Some(Random { .. }) | Some(Switch { .. }),) {
                    return RandomsInRootOrIfsBlock.into();
                }
                self.stack.push(Random {
                    rand_max,
                    chosen_value: self.is_all_match().then(|| self.rng.gen(1..=rand_max)),
                });
                Break(Ok(()))
            }
            Token::SetRandom(rand_value) => {
                if matches!(self.stack.last(), Some(Random { .. }) | Some(Switch { .. }),) {
                    return RandomsInRootOrIfsBlock.into();
                }
                self.stack.push(Random {
                    rand_max: u32::MAX,
                    chosen_value: Some(rand_value).filter(|_| self.is_all_match()),
                });
                Break(Ok(()))
            }
            Token::If(if_value) => {
                let result_check_last = match self.stack.last_mut() {
                    Some(IfBranch { .. }) => {
                        self.stack.pop();
                        OnlyOneIfInRandomBlock.into()
                    }
                    Some(ElseBranch { .. }) => {
                        self.stack.pop();
                        self.stack.push(IfBranch {
                            matching_value: u32::MAX,
                            group_previously_matched: false,
                        });
                        ElseAfterIfs.into()
                    }
                    None => return IfsInRandomBlock.into(),
                    Some(block) if !block.is_type_random() => {
                        return RandomAndSwitchCommandNotMix.into()
                    }
                    _ => Ok(()),
                };
                let Some(Random { rand_max, .. }) = self.stack.last() else {
                    return IfsInRandomBlock.into();
                };
                let result_range = (1..=*rand_max)
                    .contains(&if_value)
                    .then_some(())
                    .ok_or(ValueOutOfRange.into());
                self.stack.push(IfBranch {
                    matching_value: if_value,
                    group_previously_matched: false,
                });
                Break(result_check_last.and(result_range))
            }
            Token::ElseIf(if_value) => {
                let result_check_last = match self.stack.last() {
                    Some(Random { .. }) => {
                        self.stack.push(IfBranch {
                            matching_value: u32::MAX,
                            group_previously_matched: false,
                        });
                        Err(ElseIfAfterIf.into())
                    }
                    Some(ElseBranch { .. }) => {
                        self.stack.pop();
                        self.stack.push(IfBranch {
                            matching_value: u32::MAX,
                            group_previously_matched: false,
                        });
                        Err(ElseAfterIfs.into())
                    }
                    None => return IfsInRandomBlock.into(),
                    Some(block) if !block.is_type_random() => {
                        return RandomAndSwitchCommandNotMix.into()
                    }
                    _ => Ok(()),
                };
                let Some(
                    [Random {
                        rand_max,
                        chosen_value,
                    }, IfBranch {
                        matching_value,
                        group_previously_matched,
                    }],
                ) = self.stack.last_chunk()
                else {
                    return IfsInRandomBlock.into();
                };
                let result_unique = (if_value != *matching_value)
                    .then_some(())
                    .ok_or(ValuesInIfGroupShouldBeUnique.into());
                let result_range = (1..=*rand_max)
                    .contains(&if_value)
                    .then_some(())
                    .ok_or(ValueOutOfRange.into());
                let group_previously_matched = group_previously_matched
                    | matches!(chosen_value, Some(chosen_value) if chosen_value == matching_value);
                self.stack.pop();
                self.stack.push(IfBranch {
                    matching_value: if_value,
                    group_previously_matched,
                });
                Break(result_check_last.and(result_unique).and(result_range))
            }
            Token::Else => {
                let result_check_last = match self.stack.last() {
                    Some(Random { .. }) => {
                        self.stack.push(IfBranch {
                            matching_value: u32::MAX,
                            group_previously_matched: false,
                        });
                        ElseAfterIfs.into()
                    }
                    None => return IfsInRandomBlock.into(),
                    Some(block) if !block.is_type_random() => {
                        return RandomAndSwitchCommandNotMix.into()
                    }
                    _ => Ok(()),
                };
                let Some(
                    [Random { chosen_value, .. }, IfBranch {
                        matching_value,
                        group_previously_matched,
                        ..
                    }],
                ) = self.stack.last_chunk()
                else {
                    return IfsInRandomBlock.into();
                };
                let group_previously_matched = group_previously_matched
                    | matches!(chosen_value, Some(chosen_value) if chosen_value == matching_value);
                let else_activate = !group_previously_matched;
                self.stack.pop();
                self.stack.push(ElseBranch { else_activate });
                Break(result_check_last)
            }
            Token::EndIf => {
                let Some([Random { .. }, IfBranch { .. } | ElseBranch { .. }]) =
                    self.stack.last_chunk()
                else {
                    return EndIfAfterIfs.into();
                };
                self.stack.pop();
                Break(Ok(()))
            }
            Token::EndRandom => {
                let Some(Random { .. }) = self.stack.last() else {
                    return EndRandomAfterRandomBlock.into();
                };
                self.stack.pop();
                Break(Ok(()))
            }
            // Part: Switch
            Token::Switch(switch_max) => {
                if matches!(self.stack.last(), Some(Random { .. }) | Some(Switch { .. }),) {
                    return RandomsInRootOrIfsBlock.into();
                }
                self.stack.push(Switch {
                    switch_max,
                    chosen_value: self.is_all_match().then(|| self.rng.gen(1..=switch_max)),
                    skipping: false,
                });
                Break(Ok(()))
            }
            Token::SetSwitch(switch_value) => {
                if matches!(self.stack.last(), Some(Random { .. }) | Some(Switch { .. }),) {
                    return RandomsInRootOrIfsBlock.into();
                }
                self.stack.push(Switch {
                    switch_max: u32::MAX,
                    chosen_value: Some(switch_value).filter(|_| self.is_all_match()),
                    skipping: false,
                });
                Break(Ok(()))
            }
            Token::Case(case_value) => {
                let result_check_last = match self.stack.last_mut() {
                    None => return CasesInSwitchBlock.into(),
                    Some(block) if !block.is_type_switch() => {
                        return RandomAndSwitchCommandNotMix.into()
                    }
                    _ => Ok(()),
                };
                let Some((
                    switch_index,
                    Switch {
                        switch_max,
                        skipping: _,
                        ..
                    },
                )) = self
                    .stack
                    .iter()
                    .enumerate()
                    .rfind(|(_, block)| matches!(block, Switch { .. }))
                else {
                    return CasesInSwitchBlock.into();
                };
                let (_, cases_before) = self.stack.split_at(switch_index + 1);
                if cases_before
                    .iter()
                    .any(|block| !matches!(block, CaseBranch { .. }))
                {
                    return RandomAndSwitchCommandNotMix.into();
                }
                let result_range = (1..=*switch_max)
                    .contains(&case_value)
                    .then_some(())
                    .ok_or(ValueOutOfRange.into());
                self.stack.push(CaseBranch {
                    matching_value: case_value,
                });
                Break(result_check_last.and(result_range))
            }
            Token::Skip => {
                let result_check_last = match self.stack.last_mut() {
                    None => return CasesInSwitchBlock.into(),
                    Some(block) if !block.is_type_switch() => {
                        return RandomAndSwitchCommandNotMix.into()
                    }
                    _ => Ok(()),
                };
                let activate_skip = self.is_all_match();
                let Some(Switch { skipping, .. }) = self
                    .stack
                    .iter_mut()
                    .rfind(|block| matches!(block, Switch { .. }))
                else {
                    return CasesInSwitchBlock.into();
                };
                *skipping |= activate_skip;
                Break(result_check_last)
            }
            Token::Def => {
                let result_check_last = match self.stack.last() {
                    None => return CasesInSwitchBlock.into(),
                    Some(Switch { .. }) => DefaultAfterCase.into(),
                    Some(DefaultBranch) => OnlyOneDefaultInSwitchBlock.into(),
                    Some(block) if !block.is_type_switch() => {
                        return RandomAndSwitchCommandNotMix.into()
                    }
                    _ => Ok(()),
                };
                let Some((switch_index, Switch { .. })) = self
                    .stack
                    .iter()
                    .enumerate()
                    .rfind(|(_, block)| matches!(block, Switch { .. }))
                else {
                    return CasesInSwitchBlock.into();
                };
                self.stack.resize(switch_index + 1, DefaultBranch);
                self.stack.push(DefaultBranch);
                Break(result_check_last)
            }
            Token::EndSwitch => {
                let Some((switch_index, Switch { .. })) = self
                    .stack
                    .iter()
                    .enumerate()
                    .rfind(|(_, block)| matches!(block, Switch { .. }))
                else {
                    return EndSwitchAfterSwitchBlock.into();
                };
                self.stack.resize(switch_index, DefaultBranch);
                Break(Ok(()))
            }
            // Part: Non ControlFlow command
            _ => {
                if matches!(self.stack.last(), Some(Random { .. } | Switch { .. })) {
                    CommandInRandomBlockAndIfBlock.into()
                } else if self.is_all_match() {
                    Continue(())
                } else {
                    Break(Ok(()))
                }
            }
        }
    }
}

#[test]
fn test_random() {
    use super::rng::RngMock;
    use Token::*;
    const TOKENS: [Token; 9] = [
        Title("Outside Title"),
        Random(2),
        Title("Illegal Title"),
        If(1),
        Title("Title 1"),
        ElseIf(2),
        Title("Title 2"),
        EndIf,
        EndRandom,
    ];
    let rng = RngMock([2]);
    let mut parser = RandomParser::new(rng);
    let accepted_tokens: Vec<_> = TOKENS
        .iter()
        .filter(|token| parser.parse(token).is_continue())
        .map(ToOwned::to_owned)
        .collect();
    assert_eq!(
        accepted_tokens,
        vec![Title("Outside Title"), Title("Title 2")]
    )
}

#[test]
fn test_switch() {
    use super::rng::RngMock;
    use Token::*;
    #[rustfmt::skip]
    const TOKENS: [Token; 16] = [
        Title("Outside Title"),
        Switch(2),
            // Title("Illegal Title"),
        Case(1),
            Title("Title 1"),
        Case(2),
            Title("Title 2"),
            Switch(2),
            Case(1),
                Title("Title 2 1"),
            Case(2),
                Title("Title 2 2"),
            EndSwitch,
        Skip,
        Def,
            Title("Default Title"),
        EndSwitch,
    ];
    // 1: Find Err
    let rng = RngMock([2]);
    let mut parser = RandomParser::new(rng);
    let err_tokens: Vec<_> = TOKENS
        .iter()
        .enumerate()
        .filter_map(|(i, token)| {
            if let ControlFlow::Break(Err(error)) = parser.parse(token) {
                Some((
                    i,
                    token.to_owned(),
                    error,
                    parser.stack.len(),
                    parser.stack.last().cloned(),
                ))
            } else {
                None
            }
        })
        .collect();
    dbg!(&err_tokens);
    assert!(err_tokens.is_empty());
    // 2. Check filter
    let rng = RngMock([2]);
    let mut parser = RandomParser::new(rng);
    let accepted_tokens: Vec<_> = TOKENS
        .iter()
        .filter(|token| parser.parse(token).is_continue())
        .map(ToOwned::to_owned)
        .collect();
    assert_eq!(
        accepted_tokens,
        vec![Title("Outside Title"), Title("Title 2"), Title("Title 2 2")]
    );
    // 3. Test rng situation
    let rng = RngMock([2, 1]);
    let mut parser = RandomParser::new(rng);
    let accepted_tokens: Vec<_> = TOKENS
        .iter()
        .filter(|token| parser.parse(token).is_continue())
        .map(ToOwned::to_owned)
        .collect();
    assert_eq!(
        accepted_tokens,
        vec![
            Title("Outside Title"),
            Title("Title 2"),
            Title("Title 2 1"),
            Title("Title 2 2")
        ]
    );
}

#[test]
fn test_switch_from_nested_switch() {
    use super::rng::RngMock;
    use Token::*;
    // From tests/nested_switch.rs
    #[rustfmt::skip]
    const TOKENS: [Token; 20] = [
        Title("Outside Title"),
        Switch(2),
        Case(1),
            Title("Title 1"),
            Switch(2),
            Case(1),
                Title("Title 1 1"),
            Skip,
            Case(2),
                Title("Title 1 2"),
            Skip,
            EndSwitch,
        Skip,
        Case(2),
            Title("Title 2"),
        Skip,
        Def,
            Title("Default Title"),
        EndSwitch,
        Title("End Title"),
    ];
    let rng = RngMock([1, 2]);
    let mut parser = RandomParser::new(rng);
    let parse_results: Vec<_> = TOKENS
        .iter()
        .map(|token| {
            (
                token,
                parser.parse(token),
                (parser.stack.len(), parser.stack.last().cloned()),
            )
        })
        .collect();
    let accepted_tokens: Vec<_> = parse_results
        .iter()
        .filter(|(_, result, _)| matches!(result, ControlFlow::Continue(())))
        .map(|(token, _, _)| token)
        .map(ToOwned::to_owned)
        .map(ToOwned::to_owned)
        .collect();
    if accepted_tokens
        != vec![
            Title("Outside Title"),
            Title("Title 1"),
            Title("Title 1 2"),
            Title("End Title"),
        ]
    {
        panic!("{:#?}", parse_results)
    }
}

#[test]
fn test_random_in_switch() {
    use super::rng::RngMock;
    use Token::*;
    #[rustfmt::skip]
    const TOKENS: [Token; 25] = [
        Title("Outside Title"),
        Switch(2),
            // Title("Illegal Title"),
        Case(1),
            Title("Title 1"),
            Random(2),
            If(1),
                Title("Title 1 1"),
            ElseIf(2),
                Title("Title 1 2"),
            EndIf,
            EndRandom,
        Skip,
        Case(2),
            Title("Title 2"),
            Random(2),
            If(1),
                Title("Title 2 1"),
            ElseIf(2),
                Title("Title 2 2"),
            EndIf,
            EndRandom,
        Skip,
        Def,
            Title("Default Title"),
        EndSwitch,
    ];
    let rng = RngMock([2]);
    let mut parser = RandomParser::new(rng);
    let err_tokens: Vec<_> = TOKENS
        .iter()
        .enumerate()
        .filter_map(|(i, token)| {
            if let ControlFlow::Break(Err(error)) = parser.parse(token) {
                Some((
                    i,
                    token.to_owned(),
                    error,
                    parser.stack.len(),
                    parser.stack.last().cloned(),
                ))
            } else {
                None
            }
        })
        .collect();
    dbg!(&err_tokens);
    assert!(err_tokens.is_empty());
    let rng = RngMock([1, 2]);
    let mut parser = RandomParser::new(rng);
    let accepted_tokens: Vec<_> = TOKENS
        .iter()
        .filter(|token| parser.parse(token).is_continue())
        .map(ToOwned::to_owned)
        .collect();
    assert_eq!(
        accepted_tokens,
        vec![Title("Outside Title"), Title("Title 1"), Title("Title 1 2")]
    )
}

#[test]
fn test_switch_in_random() {
    use super::rng::RngMock;
    use Token::*;
    #[rustfmt::skip]
    const TOKENS: [Token; 14] = [
        Title("Outside Title"),
        Random(2),
            // Title("Illegal Title"),
        If(1),
            Title("Title 1"),
        Else,
            Title("Title 2"),
            Switch(2),
            Case(1),
                Title("Title 2 1"),
            Case(2),
                Title("Title 2 2"),
            EndSwitch,
        EndIf,
        EndRandom
    ];
    let rng = RngMock([2]);
    let mut parser = RandomParser::new(rng);
    let err_tokens: Vec<_> = TOKENS
        .iter()
        .enumerate()
        .filter_map(|(i, token)| {
            if let ControlFlow::Break(Err(error)) = parser.parse(token) {
                Some((
                    i,
                    token.to_owned(),
                    error,
                    parser.stack.len(),
                    parser.stack.last().cloned(),
                ))
            } else {
                None
            }
        })
        .collect();
    dbg!(&err_tokens);
    assert!(err_tokens.is_empty());
    let rng = RngMock([2]);
    let mut parser = RandomParser::new(rng);
    let accepted_tokens: Vec<_> = TOKENS
        .iter()
        .filter(|token| parser.parse(token).is_continue())
        .map(ToOwned::to_owned)
        .collect();
    assert_eq!(
        accepted_tokens,
        vec![Title("Outside Title"), Title("Title 2"), Title("Title 2 2")]
    )
}
