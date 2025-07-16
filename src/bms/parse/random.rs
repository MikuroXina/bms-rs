use std::{collections::HashMap, ops::ControlFlow::{self, *}};

use thiserror::Error;

use super::{ParseError, Result, rng::Rng};
use crate::bms::lex::token::{Token, TokenStream};

/// Parses the control flow of the token.
pub(super) fn parse_control_flow<'a>(
    token_stream: &'a TokenStream<'a>,
    rng: impl Rng,
) -> Result<Vec<&'a Token<'a>>> {
    let mut units: Vec<Unit<'a>> = Vec::new();
    todo!()
}

/// An error occurred when parsing the [`TokenStream`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum ControlFlowRule {
    #[error("In an #RANDOM - #ENDRANDOM block, there must be an #IF at the start.")]
    IfMustAtStartOfRandom,
    #[error("#CASE/#SKIP/#DEF command must be in #SWITCH - #ENDSW block")]
    CasesInSwitchBlock,
    #[error("#ELSEIF command must come after #IF block")]
    ElseIfAfterIf,
    #[error("#ELSE command must come after #IF/#ELESIF block")]
    ElseAfterIfs,
    #[error("#ENDIF command must come after #IF/#ELSEIF/#ELSE block")]
    EndIfAfterIfs,
    #[error("#ENDRANDOM command must come after #RANDOM block")]
    EndRandomAfterRandomBlock,
    #[error("#ENDSW command must come after #SWITCH block")]
    EndSwitchAfterSwitchBlock,
    #[error("#IF/#ELSEIF(#CASE) command's value is out of the range of [1, max]")]
    ValueOutOfRange,
    #[error("Unexpected token inside control flow")]
    UnexpectedTokenInFlow,
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


fn is_control_flow_token(token: &Token) -> bool {
    matches!(
        token,
        Token::Random(_)
            | Token::SetRandom(_)
            | Token::If(_)
            | Token::ElseIf(_)
            | Token::Else
            | Token::EndIf
            | Token::EndRandom
            | Token::Switch(_)
            | Token::SetSwitch(_)
            | Token::Case(_)
            | Token::Def
            | Token::Skip
            | Token::EndSwitch
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Unit<'a> {
    Token(Token<'a>),
    RandomBlock {
        max: u64,
        if_blocks: Vec<IfBlock<'a>>,
    },
    SwitchBlock {
        cases: Vec<CaseBranch<'a>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IfBlock<'a> {
    branches: HashMap<u64, IfBranch<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IfBranch<'a> {
    value: u64,
    tokens: Vec<Unit<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CaseBranch<'a> {
    value: CaseBranchValue,
    tokens: Vec<Unit<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CaseBranchValue {
    Case(u64),
    Def,
}