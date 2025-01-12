use std::ops::ControlFlow::{self, *};

use thiserror::Error;

use super::{rng::Rng, ParseError, Result};
use crate::lex::token::Token;

/// An error occurred when parsing the [`TokenStream`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum ControlFlowRule {
    #[error("#RANDOM/#SETRANDOM/#SWITCH/#SETSWITCH command must come in root of #IF/#ELSEIF/#ELSE/#CASE/#DEFAULT block")]
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
    #[error("#ENDIF command must come after #IF/#ELSEIF/#ELSE block")]
    EndIfAfterIfs,
    #[error("#ENDRANDOM command must come after #RANDOM block")]
    EndRandomAfterRandomBlock,
    #[error("#ENDRANDOM command must come after #ENDIF")]
    EndRandomAfterEndif,
    #[error("#ENDSWITCH command must come after #SWITCH block")]
    EndSwitchAfterSwitchBlock,
}

impl<T> From<ControlFlowRule> for ControlFlow<Result<T>> {
    fn from(rule: ControlFlowRule) -> Self {
        Break(Err(ParseError::ViolateControlFlowRule(rule)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum ControlFlowBlock {
    Random {
        /// It can be used for warning unreachable branch
        rand_max: u32,
        /// If the parent part cannot pass, this will be None,
        chosen_value: Option<u32>,
        /// If there is any branch has been matched. Use for #ELSE.
        matched: bool,
    },
    IfBranch {
        /// It can be used for warning unreachable `#ELESIF`
        /// If the parent part cannot pass, this will be None,
        matching_value: u32,
    },
    ElseBranch,
    Switch {
        /// It can be used for warning unreachable branch
        rand_max: u32,
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
        if matches!(
            root,
            Random {
                chosen_value: None,
                ..
            } | Switch {
                chosen_value: None,
                ..
            }
        ) {
            return false;
        }
        let flow_result = self
            .stack
            .iter()
            .skip(root_index + 1)
            .try_for_each(|block| match (root, block) {
                (
                    Random {
                        chosen_value: Some(chosen_value),
                        ..
                    },
                    IfBranch { matching_value },
                ) => Break(chosen_value == matching_value),
                (
                    Switch {
                        chosen_value: Some(chosen_value),
                        ..
                    },
                    CaseBranch { matching_value },
                ) => {
                    if chosen_value == matching_value {
                        Break(true)
                    } else {
                        Continue(())
                    }
                }
                _ => Continue(()),
            });
        matches!(flow_result, Break(true))
    }

    pub fn parse(&mut self, token: &Token) -> ControlFlow<Result<()>> {
        use ControlFlowBlock::*;
        use ControlFlowRule::*;
        match *token {
            // Part: Random
            Token::Random(rand_max) => {
                todo!();
                Break(Ok(()))
            }
            Token::SetRandom(rand_value) => {
                todo!();
                Break(Ok(()))
            }
            Token::If(if_value) => {
                todo!();
                Break(Ok(()))
            }
            Token::ElseIf(if_value) => {
                todo!();
                Break(Ok(()))
            }
            Token::Else => {
                todo!();
                Break(Ok(()))
            }
            Token::EndIf => {
                todo!();
                Break(Ok(()))
            }
            Token::EndRandom => {
                todo!();
                Break(Ok(()))
            }
            // Part: Switch
            Token::Switch(switch_max) => {
                todo!();
                Break(Ok(()))
            }
            Token::SetSwitch(switch_value) => {
                todo!();
                Break(Ok(()))
            }
            Token::Case(case_value) => {
                todo!();
                Break(Ok(()))
            }
            Token::Skip => {
                todo!();
                Break(Ok(()))
            }
            Token::Def => {
                todo!();
                Break(Ok(()))
            }
            Token::EndSwitch => {
                todo!();
                Break(Ok(()))
            }
            // Part: Non ControlFlow command
            _ => {
                if self.is_this_floor_match() {
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
    const TOKENS: [Token; 17] = [
        Title("Outside Title"),
        Switch(2),
            Title("Illegal Title"),
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
    const TOKENS: [Token; 26] = [
        Title("Outside Title"),
        Switch(2),
            Title("Illegal Title"),
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
    const TOKENS: [Token; 15] = [
        Title("Outside Title"),
        Random(2),
        Title("Illegal Title"),
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
