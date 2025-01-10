mod block;

use std::ops::ControlFlow::{self, *};

use thiserror::Error;

use self::block::{ControlFlowBlock, RandomBlock, SwitchBlock};
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
    EndifAfterIfs,
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

pub struct RandomParser<R> {
    rng: R,
    stack: Vec<ControlFlowBlock>,
}

impl<R: Rng> RandomParser<R> {
    pub fn new(rng: R) -> Self {
        Self { rng, stack: vec![] }
    }

    pub fn parse(&mut self, token: &Token) -> ControlFlow<Result<()>> {
        match *token {
            // Part: Random
            Token::Random(rand_max) => {
                let Some(parent_block) = self.stack.last() else {
                    // The First Random Level?
                    let rand_value = self.rng.gen(1..=rand_max);
                    self.stack.push(RandomBlock::new(Some(rand_value)).into());
                    return Break(Ok(()));
                };
                if parent_block.is_if_value_empty() {
                    return ControlFlowRule::RandomsInIfsBlock.into();
                }
                let rand_value = self.rng.gen(1..=rand_max);
                self.stack.push(
                    RandomBlock::new(Some(rand_value).filter(|_| parent_block.pass())).into(),
                );
                Break(Ok(()))
            }
            Token::SetRandom(rand_value) => {
                // The First Random Level
                let Some(parent_block) = self.stack.last() else {
                    self.stack.push(RandomBlock::new(Some(rand_value)).into());
                    return Break(Ok(()));
                };
                if parent_block.is_if_value_empty() {
                    return ControlFlowRule::RandomsInIfsBlock.into();
                }
                self.stack.push(
                    RandomBlock::new(Some(rand_value).filter(|_| parent_block.pass())).into(),
                );
                Break(Ok(()))
            }
            Token::If(if_value) => {
                let Some(ControlFlowBlock::Random(random_block)) = self.stack.last_mut() else {
                    return ControlFlowRule::IfsInRandomBlock.into();
                };
                if !random_block.check_clear_and_add_if_value(if_value) {
                    return ControlFlowRule::OnlyOneIfInRandomBlock.into();
                }
                Break(Ok(()))
            }
            Token::ElseIf(if_value) => {
                let Some(ControlFlowBlock::Random(random_block)) = self.stack.last_mut() else {
                    return ControlFlowRule::IfsInRandomBlock.into();
                };
                if random_block.is_if_value_empty() {
                    random_block.add_if_value(if_value);
                    return ControlFlowRule::ElseIfAfterIf.into();
                }
                random_block.clear_if_values();
                if !random_block.add_if_value(if_value) {
                    unreachable!()
                }
                Break(Ok(()))
            }
            Token::Else => {
                let Some(ControlFlowBlock::Random(random_block)) = self.stack.last_mut() else {
                    return ControlFlowRule::IfsInRandomBlock.into();
                };
                if !random_block.add_else() {
                    return ControlFlowRule::OnlyOneElseInRandomBlock.into();
                }
                Break(Ok(()))
            }
            Token::EndIf => {
                let Some(ControlFlowBlock::Random(random_block)) = self.stack.last_mut() else {
                    return ControlFlowRule::IfsInRandomBlock.into();
                };
                if random_block.is_if_value_empty() {
                    return ControlFlowRule::EndifAfterIfs.into();
                }
                random_block.reset_if();
                Break(Ok(()))
            }
            Token::EndRandom => {
                let Some(random_block) = self.stack.last_mut() else {
                    return ControlFlowRule::EndRandomAfterRandomBlock.into();
                };
                let if_closed = random_block.is_if_value_empty();
                self.stack.pop();
                if !if_closed {
                    return ControlFlowRule::EndRandomAfterEndif.into();
                }
                Break(Ok(()))
            }
            // Part: Switch
            Token::Switch(switch_max) => {
                let Some(parent_block) = self.stack.last() else {
                    // The First Random Level?
                    let switch_value = self.rng.gen(1..=switch_max);
                    self.stack.push(SwitchBlock::new(Some(switch_value)).into());
                    return Break(Ok(()));
                };
                if parent_block.is_if_value_empty() {
                    return ControlFlowRule::RandomsInIfsBlock.into();
                }
                let switch_value = self.rng.gen(1..=switch_max);
                self.stack.push(
                    SwitchBlock::new(Some(switch_value).filter(|_| parent_block.pass())).into(),
                );
                Break(Ok(()))
            }
            Token::SetSwitch(switch_value) => {
                let Some(parent_block) = self.stack.last() else {
                    // The First Random Level?
                    self.stack.push(SwitchBlock::new(Some(switch_value)).into());
                    return Break(Ok(()));
                };
                if parent_block.is_if_value_empty() {
                    return ControlFlowRule::RandomsInIfsBlock.into();
                }
                self.stack.push(
                    SwitchBlock::new(Some(switch_value).filter(|_| parent_block.pass())).into(),
                );
                Break(Ok(()))
            }
            Token::Case(case_value) => {
                let Some(ControlFlowBlock::Switch(switch_block)) = self.stack.last_mut() else {
                    return ControlFlowRule::CasesInSwitchBlock.into();
                };
                if !switch_block.add_case_value(case_value) {
                    // Pass
                }
                Break(Ok(()))
            }
            Token::Skip => {
                let Some(ControlFlowBlock::Switch(switch_block)) = self.stack.last_mut() else {
                    return ControlFlowRule::CasesInSwitchBlock.into();
                };
                switch_block.clear_case_values();
                Break(Ok(()))
            }
            Token::Def => {
                let Some(ControlFlowBlock::Switch(switch_block)) = self.stack.last_mut() else {
                    return ControlFlowRule::CasesInSwitchBlock.into();
                };
                if !switch_block.add_default() {
                    return ControlFlowRule::OnlyOneDefaultInSwitchBlock.into();
                }
                Break(Ok(()))
            }
            Token::EndSwitch => {
                let Some(_random_block) = self.stack.last_mut() else {
                    return ControlFlowRule::EndSwitchAfterSwitchBlock.into();
                };
                self.stack.pop();
                Break(Ok(()))
            }
            // Part: Non ControlFlow command
            _ => {
                if self.stack.last().is_none_or(|block| block.pass()) {
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
