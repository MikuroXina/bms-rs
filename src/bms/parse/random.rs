use std::{collections::HashMap, ops::ControlFlow::{self, *}};

use thiserror::Error;

use super::{ParseError, Result, rng::Rng};
use crate::bms::lex::token::{Token, TokenStream};

/// Parses the control flow of the token.
pub(super) fn parse_control_flow<'a>(
    token_stream: &'a TokenStream<'a>,
    mut rng: impl Rng,
) -> std::result::Result<Vec<&'a Token<'a>>, ParseError> {
    let mut units: Vec<Unit<'a>> = Vec::new();
    let val = rng.generate(0..=100);
    let mut error_list = Vec::new();
    let ast: Vec<Unit<'a>> = build_control_flow_ast(token_stream, &mut error_list);
    let tokens: Vec<&'a Token<'a>> = parse_control_flow_ast(ast, &mut error_list);
    Some(tokens).filter(|_| error_list.len() == 0).ok_or(error_list.into_iter().next().unwrap())
}

fn build_control_flow_ast<'a>(tokens: &'a TokenStream<'a>, error_list: &mut Vec<ParseError>) -> Vec<Unit<'a>> {
    let mut units: Vec<Unit<'a>> = Vec::new();
    for token in tokens.iter() {
        let unit = build_control_flow_ast_step(token);
        match unit {
            Ok(Some(unit)) => units.push(unit), 
            Ok(None) => (),
            Err(e) => error_list.push(e),
        }
    }
    units
}

fn build_control_flow_ast_step<'a>(token: &'a Token<'a>) -> std::result::Result<Option<Unit<'a>>, ParseError> {
    todo!()
}


fn parse_control_flow_ast<'a>(ast: Vec<Unit<'a>>, error_list: &mut Vec<ParseError>) -> Vec<&'a Token<'a>> {
    todo!()
}

fn parse_control_flow_ast_step<'a>(ast: Unit<'a>) -> std::result::Result<Vec<&'a Token<'a>>, ParseError> {
    todo!()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum ControlFlowRule {
    #[error("unmatched end if")]
    UnmatchedEndIf,
    #[error("unmatched end random")]
    UnmatchedEndRandom,
    #[error("unmatched end switch")]
    UnmatchedEndSwitch,
    #[error("unmatched else if")]
    UnmatchedElseIf,
    #[error("unmatched else")]
    UnmatchedElse,
}

impl From<ControlFlowRule> for ParseError {
    fn from(rule: ControlFlowRule) -> Self {
        ParseError::ViolateControlFlowRule(rule)
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
    Token(&'a Token<'a>),
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