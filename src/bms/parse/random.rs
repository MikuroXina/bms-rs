use std::{collections::HashMap, ops::ControlFlow::{self, *}};

use thiserror::Error;

use super::{ParseError, rng::Rng};
use crate::bms::lex::token::{Token, TokenStream};

/// Parses the control flow of the token.
pub(super) fn parse_control_flow<'a>(
    token_stream: &'a TokenStream<'a>,
    mut rng: impl Rng,
) -> Result<Vec<&'a Token<'a>>, ParseError> {
    // The usage of rng.
    let _val = rng.generate(0..=100);
    let mut error_list = Vec::new();
    let ast: Vec<Unit<'a>> = build_control_flow_ast(token_stream, &mut error_list);
    let tokens: Vec<&'a Token<'a>> = parse_control_flow_ast(ast, &mut rng, &mut error_list);
    Some(tokens).filter(|_| error_list.len() == 0).ok_or(error_list.into_iter().next().unwrap().into())
}

/// Control flow rules.
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

/// A unit of AST.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Unit<'a> {
    /// A token that is not a control flow token.
    Token(&'a Token<'a>),
    /// A Random block.
    RandomBlock {
        value: BlockValue,
        if_blocks: Vec<IfBlock<'a>>,
    },
    /// A Switch block.
    SwitchBlock {
        value: BlockValue,
        cases: Vec<CaseBranch<'a>>,
    },
}

/// The value of a Random/Switch block.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BlockValue {
    /// For Random/Switch, value ranges in [1, max].
    /// IfBranch value must ranges in [1, max].
    Random {
        max: u64,
    },
    /// For SetRandom/SetSwitch.
    /// IfBranch value has no limit.
    Set {
        value: u64,
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

/// Checks if a token is a control flow token.
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

fn build_control_flow_ast<'a>(tokens: &'a TokenStream<'a>, error_list: &mut Vec<ControlFlowRule>) -> Vec<Unit<'a>> {
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

fn build_control_flow_ast_step<'a>(token: &'a Token<'a>) -> Result<Option<Unit<'a>>, ControlFlowRule> {
    todo!()
}


fn parse_control_flow_ast<'a>(ast: Vec<Unit<'a>>, rng: &mut impl Rng, error_list: &mut Vec<ControlFlowRule>) -> Vec<&'a Token<'a>> {
    let mut tokens: Vec<&'a Token<'a>> = Vec::new();
    for unit in ast.iter() {
        let token = parse_control_flow_ast_step(unit, rng);
        match token {
            Ok(Some(unit)) => tokens.extend(unit.into_iter()), 
            Ok(None) => (),
            Err(e) => error_list.push(e),
        }
    }
    tokens
}

fn parse_control_flow_ast_step<'a>(ast: &Unit<'a>, rng: &mut impl Rng) -> Result<Option<Vec<&'a Token<'a>>>, ControlFlowRule> {
    todo!()
}