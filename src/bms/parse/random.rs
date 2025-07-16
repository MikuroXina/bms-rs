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