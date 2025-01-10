use std::ops::ControlFlow::{self, *};

use thiserror::Error;

use super::{rng::Rng, ParseError, Result};
use crate::lex::token::Token;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ClauseState {
    Random(u32),
    If(bool),
}

/// An error occurred when parsing the [`TokenStream`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum ControlFlowRule {
    #[error("#RANDOM/#SETRANDOM command must come in root of #IF block")]
    RandomInIfBlock,
    #[error("#IF command must be in #RANDOM - #ENDRANDOM block")]
    IfInRandomBlock,
    #[error("#ELSEIF command must come after #IF block")]
    ElseIfAfterIf,
    #[error("#ENDIF command must come after #IF or #ELSEIF block")]
    EndifAfterIf,
    #[error("#ENDRANDOM command must come after #RANDOM block")]
    EndRandomAfterRandomBlock,
}

pub struct RandomParser<R> {
    rng: R,
    random_stack: Vec<ClauseState>,
}

impl<R: Rng> RandomParser<R> {
    pub fn new(rng: R) -> Self {
        Self {
            rng,
            random_stack: vec![],
        }
    }

    pub fn parse(&mut self, token: &Token) -> ControlFlow<Result<()>> {
        match *token {
            // Part: Random
            Token::Random(rand_max) => {
                if let Some(&ClauseState::Random(_)) = self.random_stack.last() {
                    Break(Err(ParseError::ViolateControlFlowRule(
                        ControlFlowRule::RandomInIfBlock,
                    )))
                } else if let Some(ClauseState::If(false)) = self.random_stack.last() {
                    self.random_stack.push(ClauseState::Random(0));
                    Break(Ok(()))
                } else {
                    self.random_stack
                        .push(ClauseState::Random(self.rng.gen(1..=rand_max)));
                    Break(Ok(()))
                }
            }
            Token::SetRandom(rand_value) => {
                if let Some(&ClauseState::Random(_)) = self.random_stack.last() {
                    Break(Err(ParseError::ViolateControlFlowRule(
                        ControlFlowRule::RandomInIfBlock,
                    )))
                } else if let Some(ClauseState::If(false)) = self.random_stack.last() {
                    self.random_stack.push(ClauseState::Random(0));
                    Break(Ok(()))
                } else {
                    self.random_stack.push(ClauseState::Random(rand_value));
                    Break(Ok(()))
                }
            }
            Token::If(rand_target) => {
                if let Some(&ClauseState::Random(rand)) = self.random_stack.last() {
                    self.random_stack.push(ClauseState::If(rand_target == rand));
                    Break(Ok(()))
                } else {
                    Break(Err(ParseError::ViolateControlFlowRule(
                        ControlFlowRule::IfInRandomBlock,
                    )))
                }
            }
            Token::ElseIf(rand_target) => {
                if let Some(ClauseState::If(_)) = self.random_stack.last() {
                    self.random_stack.pop();
                    let rand = match self.random_stack.last().unwrap() {
                        &ClauseState::Random(rand) => rand,
                        ClauseState::If(_) => unreachable!(),
                    };
                    self.random_stack.push(ClauseState::If(rand_target == rand));
                    Break(Ok(()))
                } else {
                    Break(Err(ParseError::ViolateControlFlowRule(
                        ControlFlowRule::ElseIfAfterIf,
                    )))
                }
            }
            Token::Else => {
                todo!()
            }
            Token::EndIf => {
                if let Some(ClauseState::If(_)) = self.random_stack.last() {
                    self.random_stack.pop();
                    Break(Ok(()))
                } else {
                    Break(Err(ParseError::ViolateControlFlowRule(
                        ControlFlowRule::EndifAfterIf,
                    )))
                }
            }
            Token::EndRandom => {
                if let Some(&ClauseState::Random(_)) = self.random_stack.last() {
                    self.random_stack.pop();
                    Break(Ok(()))
                } else {
                    Break(Err(ParseError::ViolateControlFlowRule(
                        ControlFlowRule::EndRandomAfterRandomBlock,
                    )))
                }
            }
            // Part: Switch
            Token::Switch(switch_max) => {
                dbg!(switch_max);
                todo!()
            }
            Token::SetSwitch(switch_value) => {
                dbg!(switch_value);
                todo!()
            }
            Token::Case(case_value) => {
                dbg!(case_value);
                todo!()
            }
            Token::Skip => {
                todo!()
            }
            Token::Def => {
                todo!()
            }
            Token::EndSwitch => {
                todo!()
            }
            // Part: Non ControlFlow command
            _ => {
                if let Some(ClauseState::Random(_) | ClauseState::If(false)) =
                    self.random_stack.last()
                {
                    Break(Ok(()))
                } else {
                    Continue(())
                }
            }
        }
    }
}
