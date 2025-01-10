use std::ops::ControlFlow::{self, *};

use super::{rng::Rng, ParseError, Result};
use crate::lex::token::Token;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ClauseState {
    Random(u32),
    If(bool),
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
            Token::If(rand_target) => {
                if let Some(&ClauseState::Random(rand)) = self.random_stack.last() {
                    self.random_stack.push(ClauseState::If(rand_target == rand));
                    Break(Ok(()))
                } else {
                    Break(Err(ParseError::SyntaxError(
                        "#IF command must be in #RANDOM - #ENDRANDOM block".into(),
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
                    Break(Err(ParseError::SyntaxError(
                        "#ELSEIF command must come after #IF block".into(),
                    )))
                }
            }
            Token::EndIf => {
                if let Some(ClauseState::If(_)) = self.random_stack.last() {
                    self.random_stack.pop();
                    Break(Ok(()))
                } else {
                    Break(Err(ParseError::SyntaxError(
                        "#ENDIF command must come after #IF or #ELSEIF block".into(),
                    )))
                }
            }
            Token::Random(rand_max) => {
                if let Some(&ClauseState::Random(_)) = self.random_stack.last() {
                    Break(Err(ParseError::SyntaxError(
                        "#RANDOM command must come in root or #IF block".into(),
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
            Token::EndRandom => {
                if let Some(&ClauseState::Random(_)) = self.random_stack.last() {
                    self.random_stack.pop();
                    Break(Ok(()))
                } else {
                    Break(Err(ParseError::SyntaxError(
                        "#ENDRANDOM command must come after #RANDOM block".into(),
                    )))
                }
            }
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
