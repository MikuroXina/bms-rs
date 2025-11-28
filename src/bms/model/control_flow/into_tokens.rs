//! Trait for converting a value into a vector of tokens.
//!
//! This trait provides a unified interface for control-flow models and
//! other token-producing structures to convert themselves into a sequence
//! of lex `Token`s.

use super::{ControlFlowValue, IfChainEntry, Random, Switch, TokenUnit};
use crate::bms::lex::token::Token;

/// Converts a value into a vector of lex `Token`s.
///
/// Types implementing this trait can be serialized into the token stream
/// used by the BMS parser and unparser.
pub trait IntoTokens<'a> {
    /// Convert `self` into a vector of lex `Token`s.
    fn into_tokens(self) -> Vec<Token<'a>>;
}

impl<'a> IntoTokens<'a> for TokenUnit<'a> {
    fn into_tokens(self) -> Vec<Token<'a>> {
        match self {
            TokenUnit::Random(r) => IntoTokens::into_tokens(r),
            TokenUnit::Switch(s) => IntoTokens::into_tokens(s),
            TokenUnit::Tokens(v) => v.into_iter().map(Token::from).collect(),
        }
    }
}

impl<'a> IntoTokens<'a> for Random<'a> {
    fn into_tokens(self) -> Vec<Token<'a>> {
        let mut out = Vec::new();
        match &self.value {
            ControlFlowValue::GenMax(max) => out.push(Token::header("RANDOM", max.to_string())),
            ControlFlowValue::Set(val) => out.push(Token::header("SETRANDOM", val.to_string())),
        }

        self.branches.into_iter().for_each(|branch| {
            out.push(Token::header("IF", branch.condition.to_string()));
            out.extend(
                branch
                    .head_units
                    .into_iter()
                    .flat_map(IntoTokens::into_tokens),
            );

            let mut node = branch.chain;
            loop {
                match node {
                    IfChainEntry::ElseIf { cond, units, next } => {
                        out.push(Token::header("ELSEIF", cond.to_string()));
                        out.extend(units.into_iter().flat_map(IntoTokens::into_tokens));
                        node = *next;
                    }
                    IfChainEntry::Else { units } => {
                        out.push(Token::header("ELSE", ""));
                        out.extend(units.into_iter().flat_map(IntoTokens::into_tokens));
                        break;
                    }
                    IfChainEntry::EndIf => break,
                }
            }

            out.push(Token::header("ENDIF", ""));
        });

        out.push(Token::header("ENDRANDOM", ""));

        out
    }
}

impl<'a> IntoTokens<'a> for Switch<'a> {
    fn into_tokens(self) -> Vec<Token<'a>> {
        let mut out = Vec::new();
        match &self.value {
            ControlFlowValue::GenMax(max) => out.push(Token::header("SWITCH", max.to_string())),
            ControlFlowValue::Set(val) => out.push(Token::header("SETSWITCH", val.to_string())),
        }

        self.cases.into_iter().for_each(|case| {
            out.extend(
                std::iter::once(case.condition.map_or_else(
                    || Token::header("DEF", ""),
                    |cond| Token::header("CASE", cond.to_string()),
                ))
                .chain(case.units.into_iter().flat_map(IntoTokens::into_tokens))
                .chain(case.skip.then(|| Token::header("SKIP", ""))),
            );
        });

        out.push(Token::header("ENDSW", ""));

        out
    }
}
