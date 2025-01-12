use std::ops::ControlFlow::{self, *};

use thiserror::Error;

use super::{rng::Rng, ParseError, Result};
use crate::lex::token::Token;

/// An error occurred when parsing the [`TokenStream`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum ControlFlowRule {
    #[error(
        "Other command should be in a #RANDOM(#SWITCH) and its #IF/#ELSE/#ELSE(#CASE/#DEFAULT) block"
    )]
    CommandInRandomBlockAndIfBlock,
    #[error("#RANDOM block and #SWITCH block commands should not mixed.")]
    RandomAndSwitchCommandNotMix,
    #[error("#RANDOM/#SETRANDOM(#SWITCH/#SETSWITCH) command must come in root of #IF/#ELSEIF/#ELSE(#CASE/#DEFAULT) block")]
    RandomsInIfsBlock,
    #[error("#IF/#ELSEIF/#ELSE/#ENDIF command must be in #RANDOM - #ENDRANDOM block")]
    IfsInRandomBlock,
    #[error("#CASE/#SKIP/#DEF command must be in #SWITCH - #ENDSWITCH block")]
    CasesInSwitchBlock,
    #[error("Only 1 #IF command is allowed in a #RANDOM - #ENDRANDOM block")]
    OnlyOneIfInRandomBlock,
    #[error("Only 1 #ELSE command is allowed in a #RANDOM - #ENDRANDOM block")]
    OnlyOneElseInRandomBlock,
    #[error("Only 1 #DEFAULT command is allowed in a #SWITCH - #ENDSWITCH block")]
    OnlyOneDefaultInSwitchBlock,
    #[error("#ELSEIF command must come after #IF block")]
    ElseIfAfterIf,
    #[error("#ELSE command must come after #IF/#ELESIF block")]
    ElseAfterIfOrElseIf,
    #[error("#ENDIF command must come after #IF/#ELSEIF/#ELSE block")]
    EndIfAfterIfs,
    #[error("#ENDRANDOM command must come after #RANDOM block")]
    EndRandomAfterRandomBlock,
    #[error("#ENDRANDOM command must come after #ENDIF")]
    EndRandomAfterEndif,
    #[error("#ENDSWITCH command must come after #SWITCH block")]
    EndSwitchAfterSwitchBlock,
    #[error("#IF/#ELSEIF(#CASE) command's value is out of the range of [1, max]")]
    IfsValueOutOfRange,
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
        /// If there is any if/elseif branch has been matched in this #IF group. Used by #ELSE.
        group_previous_matched: bool,
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

pub struct RandomParser<R> {
    rng: R,
    stack: Vec<ControlFlowBlock>,
}

impl<R: Rng> RandomParser<R> {
    pub fn new(rng: R) -> Self {
        Self { rng, stack: vec![] }
    }

    fn is_this_floor_match(&self) -> bool {
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
                    group_previous_matched,
                } => !group_previous_matched && matching_value == chosen_value,
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
                    return RandomsInIfsBlock.into();
                }
                self.stack.push(Random {
                    rand_max,
                    chosen_value: self
                        .is_this_floor_match()
                        .then(|| self.rng.gen(1..=rand_max)),
                });
                Break(Ok(()))
            }
            Token::SetRandom(rand_value) => {
                if matches!(self.stack.last(), Some(Random { .. }) | Some(Switch { .. }),) {
                    return RandomsInIfsBlock.into();
                }
                self.stack.push(Random {
                    rand_max: u32::MAX,
                    chosen_value: Some(rand_value).filter(|_| self.is_this_floor_match()),
                });
                Break(Ok(()))
            }
            Token::If(if_value) => {
                let Some(Random { rand_max, .. }) = self.stack.last() else {
                    return IfsInRandomBlock.into();
                };
                let result = if !(1..=*rand_max).contains(&if_value) {
                    Err(IfsValueOutOfRange.into())
                } else {
                    Ok(())
                };
                self.stack.push(IfBranch {
                    matching_value: if_value,
                    group_previous_matched: false,
                });
                Break(result)
            }
            Token::ElseIf(if_value) => {
                let Some(
                    [Random {
                        rand_max,
                        chosen_value,
                    }, IfBranch {
                        matching_value,
                        group_previous_matched,
                    }],
                ) = self.stack.last_chunk()
                else {
                    return ElseIfAfterIf.into();
                };
                let result = if if_value == *matching_value {
                    Err(ValuesInIfGroupShouldBeUnique.into())
                } else if !(1..=*rand_max).contains(&if_value) {
                    Err(IfsValueOutOfRange.into())
                } else {
                    Ok(())
                };
                let group_previous_matched = group_previous_matched
                    | matches!(chosen_value, Some(chosen_value) if chosen_value == matching_value);
                self.stack.pop();
                self.stack.push(IfBranch {
                    matching_value: if_value,
                    group_previous_matched,
                });
                Break(result)
            }
            Token::Else => {
                let Some(
                    [Random { .. }, IfBranch {
                        group_previous_matched,
                        ..
                    }],
                ) = self.stack.last_chunk()
                else {
                    return ElseAfterIfOrElseIf.into();
                };
                let else_activate = !group_previous_matched;
                self.stack.pop();
                self.stack.push(ElseBranch { else_activate });
                Break(Ok(()))
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
                    return RandomsInIfsBlock.into();
                }
                self.stack.push(Switch {
                    switch_max,
                    chosen_value: self
                        .is_this_floor_match()
                        .then(|| self.rng.gen(1..=switch_max)),
                    skipping: false,
                });
                Break(Ok(()))
            }
            Token::SetSwitch(switch_value) => {
                if matches!(self.stack.last(), Some(Random { .. }) | Some(Switch { .. }),) {
                    return RandomsInIfsBlock.into();
                }
                self.stack.push(Switch {
                    switch_max: u32::MAX,
                    chosen_value: Some(switch_value).filter(|_| self.is_this_floor_match()),
                    skipping: false,
                });
                Break(Ok(()))
            }
            Token::Case(case_value) => {
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
                let result = if !(1..=*switch_max).contains(&case_value) {
                    Err(IfsValueOutOfRange.into())
                } else {
                    Ok(())
                };
                self.stack.push(CaseBranch {
                    matching_value: case_value,
                });
                Break(result)
            }
            Token::Skip => {
                let Some(Switch { skipping, .. }) = self
                    .stack
                    .iter_mut()
                    .rfind(|block| matches!(block, Switch { .. }))
                else {
                    return CasesInSwitchBlock.into();
                };
                *skipping = true;
                Break(Ok(()))
            }
            Token::Def => {
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
                Break(Ok(()))
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
                } else if self.is_this_floor_match() {
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
