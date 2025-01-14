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
        /// Whether found matching case or default
        matched: bool,
        /// Whether skipping tokens until `#ENDSW`
        skipping: bool,
    },
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
        use ControlFlowRule::*;

        if let Some(&ControlFlowBlock::Random {
            rand_max,
            chosen_value,
        }) = self.stack.last()
        {
            // expected only if blocks and end random
            if *token == Token::EndRandom {
                self.stack.pop();
                return Break(Ok(()));
            }
            let &Token::If(matching_value) = token else {
                return IfsInRandomBlock.into();
            };
            if matching_value > rand_max {
                return ValueOutOfRange.into();
            }
            self.stack.push(ControlFlowBlock::IfBranch {
                matching_value,
                group_previously_matched: chosen_value == Some(matching_value),
            });
            return Break(Ok(()));
        }

        let flow_enabled = if let Some(ControlFlowBlock::Switch {
            chosen_value,
            switch_max,
            matched,
            skipping,
        }) = self.stack.last_mut()
        {
            // expected cases, default and other commands
            if *token == Token::EndSwitch {
                self.stack.pop();
                return Break(Ok(()));
            }
            if *skipping {
                // skipping until end of switch
                return Break(Ok(()));
            }
            if !*matched {
                // ignoring until matching case or default
                if let Token::Case(matching_value) = token {
                    if matching_value > switch_max {
                        return ValueOutOfRange.into();
                    }
                    if Some(matching_value) == chosen_value.as_ref() {
                        *matched = true;
                    }
                }
                if *token == Token::Def {
                    *matched = true;
                }
            } else if *token == Token::Skip {
                // skip only if matched
                *skipping = true;
                return Break(Ok(()));
            }
            if matches!(token, Token::Case(_) | Token::Def) {
                return Break(Ok(()));
            }
            *matched
        } else {
            // another control flow
            true
        };

        let flow_enabled = flow_enabled
            && if let Some(
                [ControlFlowBlock::Random {
                    rand_max,
                    chosen_value,
                }, ControlFlowBlock::IfBranch {
                    matching_value,
                    group_previously_matched,
                }],
            ) = self.stack.last_chunk_mut()
            {
                // expected else if, else, end if and other commands
                if *token == Token::EndIf {
                    self.stack.pop();
                    return Break(Ok(()));
                }
                let matched = *group_previously_matched;
                if let &Token::ElseIf(else_matching_value) = token {
                    if &else_matching_value > rand_max {
                        return ValueOutOfRange.into();
                    }
                    self.stack.pop();
                    self.stack.push(ControlFlowBlock::IfBranch {
                        matching_value: else_matching_value,
                        group_previously_matched: matched,
                    });
                    return Break(Ok(()));
                }
                if *token == Token::Else {
                    self.stack.pop();
                    self.stack.push(ControlFlowBlock::ElseBranch {
                        else_activate: !matched,
                    });
                    return Break(Ok(()));
                }
                if chosen_value.is_some_and(|chosen| &chosen == matching_value) {
                    // activate only this matched branch
                    *group_previously_matched = true;
                    true
                } else {
                    // the control flow is disabled
                    false
                }
            } else {
                // another control flow
                true
            };

        if let Some(&ControlFlowBlock::ElseBranch { else_activate }) = self.stack.last() {
            if let Token::ElseIf(_) = token {
                return ElseIfAfterIf.into();
            }
            if *token == Token::EndIf {
                self.stack.pop();
                return Break(Ok(()));
            }
            if !else_activate {
                return Break(Ok(()));
            }
        }

        // expected block starting commands below

        if let &Token::Random(max) = token {
            self.stack.push(ControlFlowBlock::Random {
                rand_max: max,
                chosen_value: flow_enabled.then(|| self.rng.gen(1..=max)),
            });
            return Break(Ok(()));
        }
        if let &Token::SetRandom(chosen) = token {
            self.stack.push(ControlFlowBlock::Random {
                rand_max: u32::MAX,
                chosen_value: flow_enabled.then_some(chosen),
            });
            return Break(Ok(()));
        }

        if let &Token::Switch(max) = token {
            self.stack.push(ControlFlowBlock::Switch {
                switch_max: max,
                chosen_value: flow_enabled.then(|| self.rng.gen(1..=max)),
                matched: false,
                skipping: false,
            });
            return Break(Ok(()));
        }
        if let &Token::SetSwitch(chosen) = token {
            self.stack.push(ControlFlowBlock::Switch {
                switch_max: u32::MAX,
                chosen_value: flow_enabled.then_some(chosen),
                matched: false,
                skipping: false,
            });
            return Break(Ok(()));
        }

        if matches!(
            token,
            Token::If(_) | Token::ElseIf(_) | Token::Else | Token::EndIf
        ) {
            return IfsInRandomBlock.into();
        }
        if matches!(token, Token::Case(_) | Token::Def) {
            return CasesInSwitchBlock.into();
        }

        if flow_enabled {
            Continue(())
        } else {
            Break(Ok(()))
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
