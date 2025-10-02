//! Random control flow parsing for BMS format.
//!
//! This module provides functionality for parsing and executing random control flow constructs
//! in BMS (Be-Music Source) files. It handles `#RANDOM`, `#IF`, `#ELSEIF`, `#ELSE`, `#ENDIF`,
//! `#SWITCH`, `#CASE`, `#DEF`, `#SKIP`, and `#ENDSWITCH` directives.
//!
//! # Overview
//!
//! The random module consists of three main components:
//!
//! - **AST Building** (`ast_build`): Converts token streams into Abstract Syntax Trees (AST)
//! - **AST Parsing** (`ast_parse`): Executes the AST using a random number generator
//! - **RNG** (`rng`): Provides random number generation capabilities
//!
//! All errors are collected as warnings and returned alongside the parsed tokens,
//! allowing the parser to continue processing while providing detailed error information.

mod ast_build;
mod ast_extract;
mod ast_parse;
pub mod rng;
pub mod structure;

use core::ops::RangeInclusive;
use num::BigUint;
use rng::Rng;
use thiserror::Error;

use crate::bms::{
    command::mixin::SourceRangeMixin,
    lex::{TokenRefStream, TokenStream, token::TokenWithRange},
};

use self::{ast_build::build_control_flow_ast, ast_parse::parse_control_flow_ast, structure::Unit};

/// The root of the AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstRoot<'a> {
    /// The units of the AST.
    pub units: Vec<Unit<'a>>,
}

/// The output of building the AST.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct AstBuildOutput<'a> {
    /// The units of the AST.
    pub root: AstRoot<'a>,
    /// The errors that occurred during building.
    pub ast_build_warnings: Vec<AstBuildWarningWithRange>,
}

impl<'a> AstRoot<'a> {
    /// Builds the AST from a token stream.
    pub fn from_token_stream(
        token_stream: impl IntoIterator<Item = &'a TokenWithRange<'a>>,
    ) -> AstBuildOutput<'a> {
        let mut token_iter = token_stream.into_iter().peekable();
        let (units, errors) = build_control_flow_ast(&mut token_iter);
        AstBuildOutput {
            root: AstRoot { units },
            ast_build_warnings: errors,
        }
    }
}

/// The output of parsing the AST.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct AstParseOutput<'a> {
    /// The tokens that were parsed.
    pub token_refs: TokenRefStream<'a>,
}

impl<'a> AstRoot<'a> {
    /// Parses the AST using a random number generator.
    pub fn parse(self, mut rng: impl Rng) -> AstParseOutput<'a> {
        let mut ast_iter = self.units.into_iter().peekable();
        let (tokens, _warnings) = parse_control_flow_ast(&mut ast_iter, &mut rng);
        AstParseOutput {
            token_refs: TokenRefStream { token_refs: tokens },
        }
    }
}

impl<'a> AstRoot<'a> {
    /// Extracts all tokens from the AST and returns them as a [`TokenStream`].
    /// This function flattens the AST structure and returns ALL tokens contained in the AST,
    /// including all branches in Random and Switch blocks. This serves as the inverse of
    /// [`AstRoot::from_token_stream`].
    #[must_use]
    pub fn extract(self) -> TokenStream<'a> {
        let tokens = ast_extract::extract_units(self.units);
        TokenStream { tokens }
    }
}

/// Control flow parsing errors and warnings.
///
/// This enum defines all possible errors that can occur during BMS control flow parsing.
/// Each variant represents a specific type of control flow violation or malformed construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AstBuildWarning {
    /// An `#ENDIF` token was encountered without a corresponding `#IF` token.
    #[error("unmatched end if")]
    UnmatchedEndIf,
    /// An `#ENDRANDOM` token was encountered without a corresponding `#RANDOM` token.
    #[error("unmatched end random")]
    UnmatchedEndRandom,
    /// An `#ENDSWITCH` token was encountered without a corresponding `#SWITCH` token.
    #[error("unmatched end switch")]
    UnmatchedEndSwitch,
    /// An `#ELSEIF` token was encountered without a corresponding `#IF` token.
    #[error("unmatched else if")]
    UnmatchedElseIf,
    /// An `#ELSE` token was encountered without a corresponding `#IF` token.
    #[error("unmatched else")]
    UnmatchedElse,
    /// A duplicate `#IF` branch value was found in a random block.
    #[error("duplicate if branch value in random block")]
    RandomDuplicateIfBranchValue,
    /// An `#IF` branch value exceeds the maximum value of its random block.
    #[error("if branch value out of range in random block")]
    RandomIfBranchValueOutOfRange,
    /// A duplicate `#CASE` value was found in a switch block.
    #[error("duplicate case value in switch block")]
    SwitchDuplicateCaseValue,
    /// A `#CASE` value exceeds the maximum value of its switch block.
    #[error("case value out of range in switch block")]
    SwitchCaseValueOutOfRange,
    /// Multiple `#DEF` branches were found in the same switch block.
    #[error("duplicate def branch in switch block")]
    SwitchDuplicateDef,
    /// A `#SKIP` token was encountered outside of a switch block.
    #[error("unmatched skip")]
    UnmatchedSkip,
    /// A `#CASE` token was encountered outside of a switch block.
    #[error("unmatched case")]
    UnmatchedCase,
    /// A `#DEF` token was encountered outside of a switch block.
    #[error("unmatched def")]
    UnmatchedDef,
}

/// A [`AstBuildWarning`] type with position information.
pub type AstBuildWarningWithRange = SourceRangeMixin<AstBuildWarning>;

/// Warnings that occurred during AST parsing (execution with RNG).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AstParseWarning {
    /// [`Rng`] generated a value outside the required [`RangeInclusive`] for a [`Unit::RandomBlock`].
    #[error("random generated value out of range: expected {expected:?}, got {actual}")]
    RandomGeneratedValueOutOfRange {
        /// The expected range of the [`Unit::RandomBlock`].
        expected: SourceRangeMixin<RangeInclusive<BigUint>>,
        /// The actual value generated by the [`Rng`].
        actual: BigUint,
    },
    /// [`Rng`] generated a value outside the required [`RangeInclusive`] for a [`Unit::SwitchBlock`].
    #[error("switch generated value out of range: expected {expected:?}, got {actual}")]
    SwitchGeneratedValueOutOfRange {
        /// The expected range of the [`Unit::SwitchBlock`].
        expected: SourceRangeMixin<RangeInclusive<BigUint>>,
        /// The actual value generated by the [`Rng`].
        actual: BigUint,
    },
}

/// A [`AstParseWarning`] type with position information.
pub type AstParseWarningWithRange = SourceRangeMixin<AstParseWarning>;

impl<'a> AstRoot<'a> {
    /// Parses the AST and collects AST-parse warnings.
    pub fn parse_with_warnings(
        self,
        mut rng: impl Rng,
    ) -> (AstParseOutput<'a>, Vec<AstParseWarningWithRange>) {
        let mut ast_iter = self.units.into_iter().peekable();
        let (tokens, warnings) = parse_control_flow_ast(&mut ast_iter, &mut rng);
        (
            AstParseOutput {
                token_refs: TokenRefStream { token_refs: tokens },
            },
            warnings,
        )
    }
}

#[cfg(test)]
mod tests {
    use core::ops::RangeInclusive;

    use num::BigUint;

    use super::*;
    use crate::bms::{
        ast::structure::{CaseBranch, CaseBranchValue, Unit},
        command::mixin::SourceRangeMixinExt,
        lex::token::Token,
    };

    struct DummyRng;
    impl Rng for DummyRng {
        fn generate(&mut self, range: RangeInclusive<BigUint>) -> BigUint {
            // Always return the maximum value
            range.end().clone()
        }
    }

    #[test]
    fn test_switch_nested_switch_case() {
        use crate::bms::lex::token::ControlFlow as CF;
        let tokens = vec![
            Token::header("TITLE", "11000000"),
            Token::ControlFlow(CF::Switch(BigUint::from(2u32))),
            Token::ControlFlow(CF::Case(BigUint::from(1u32))),
            Token::header("TITLE", "00220000"),
            Token::ControlFlow(CF::Random(BigUint::from(2u32))),
            Token::ControlFlow(CF::If(BigUint::from(1u32))),
            Token::header("TITLE", "00550000"),
            Token::ControlFlow(CF::ElseIf(BigUint::from(2u32))),
            Token::header("TITLE", "00006600"),
            Token::ControlFlow(CF::EndIf),
            Token::ControlFlow(CF::EndRandom),
            Token::ControlFlow(CF::Skip),
            Token::ControlFlow(CF::Case(BigUint::from(2u32))),
            Token::header("TITLE", "00003300"),
            Token::ControlFlow(CF::Skip),
            Token::ControlFlow(CF::EndSwitch),
            Token::header("TITLE", "00000044"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
        println!("AST structure: {ast:#?}");
        let Some(Unit::SwitchBlock { cases, .. }) =
            ast.iter().find(|u| matches!(u, Unit::SwitchBlock { .. }))
        else {
            panic!("AST structure error");
        };
        let Some(case1) = cases
            .iter()
            .find(|c| c.value.content() == &CaseBranchValue::Case(BigUint::from(1u64)))
        else {
            panic!("Case(1) not found");
        };
        println!("Case(1) tokens: {:#?}", case1.units);
        assert_eq!(errors, vec![]);
        assert!(matches!(&ast[0], Unit::TokenWithRange(_))); // 11000000
        assert!(matches!(&ast[1], Unit::SwitchBlock { .. }));
        assert!(matches!(&ast[2], Unit::TokenWithRange(_))); // 00000044
        let Unit::SwitchBlock { cases, .. } = &ast[1] else {
            panic!("AST structure error");
        };
        let Some(CaseBranch { units: tokens, .. }) = cases
            .iter()
            .find(|c| c.value.content() == &CaseBranchValue::Case(BigUint::from(1u64)))
        else {
            panic!("Case(1) not found");
        };
        assert!(matches!(&tokens[0], Unit::TokenWithRange(_))); // 00220000
        assert!(matches!(&tokens[1], Unit::RandomBlock { .. }));
        let Unit::RandomBlock { if_blocks, .. } = &tokens[1] else {
            panic!("RandomBlock not found");
        };
        let if_block = &if_blocks[0];
        assert!(
            if_block
                .branches
                .get(&BigUint::from(1u64))
                .unwrap()
                .content()
                .iter()
                .filter_map(|u| match u {
                    Unit::TokenWithRange(token) => Some(token),
                    _ => None,
                })
                .any(|u| u.content() == &Token::header("TITLE", "00550000"))
        );
        assert!(
            if_block
                .branches
                .get(&BigUint::from(2u64))
                .unwrap()
                .content()
                .iter()
                .filter_map(|u| match u {
                    Unit::TokenWithRange(token) => Some(token),
                    _ => None,
                })
                .any(|u| u.content() == &Token::header("TITLE", "00006600"))
        );
        let Some(CaseBranch { units: tokens, .. }) = cases
            .iter()
            .find(|c| c.value.content() == &CaseBranchValue::Case(BigUint::from(2u64)))
        else {
            panic!("Case(2) not found");
        };
        assert!({
            let Unit::TokenWithRange(token) = &tokens[0] else {
                panic!("Unit::TokenWithRange expected, got: {tokens:?}");
            };
            token.content() == &Token::header("TITLE", "00003300")
        });
        let mut rng = DummyRng;
        let mut ast_iter = ast.into_iter().peekable();
        let (tokens, _warnings) = parse_control_flow_ast(&mut ast_iter, &mut rng);
        let expected = ["11000000", "00003300", "00000044"];
        assert_eq!(tokens.len(), 3);
        for (i, t) in tokens.iter().enumerate() {
            assert!(t.content() == &Token::header("TITLE", expected[i]));
        }
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn test_switch_insane_tokenized() {
        use crate::bms::lex::token::ControlFlow as CF;
        let tokens = vec![
            Token::ControlFlow(CF::Switch(BigUint::from(5u32))),
            Token::ControlFlow(CF::Def),
            Token::header("TITLE", "0055"),
            Token::ControlFlow(CF::Skip),
            Token::ControlFlow(CF::Case(BigUint::from(1u32))),
            Token::header("TITLE", "0100000000000000"),
            Token::ControlFlow(CF::Random(BigUint::from(2u32))),
            Token::ControlFlow(CF::If(BigUint::from(1u32))),
            Token::header("TITLE", "04"),
            Token::ControlFlow(CF::Else),
            Token::header("TITLE", "05"),
            Token::ControlFlow(CF::EndIf),
            // Missing EndRandom!!!
            Token::ControlFlow(CF::Case(BigUint::from(2u32))),
            Token::header("TITLE", "0200000000000000"),
            Token::ControlFlow(CF::Skip),
            Token::ControlFlow(CF::Case(BigUint::from(3u32))),
            Token::header("TITLE", "0300000000000000"),
            Token::ControlFlow(CF::Switch(BigUint::from(2u32))),
            Token::ControlFlow(CF::Case(BigUint::from(1u32))),
            Token::header("TITLE", "1111"),
            Token::ControlFlow(CF::Skip),
            Token::ControlFlow(CF::Case(BigUint::from(2u32))),
            Token::header("TITLE", "2222"),
            Token::ControlFlow(CF::Skip),
            Token::ControlFlow(CF::EndSwitch),
            Token::ControlFlow(CF::Skip),
            Token::ControlFlow(CF::EndSwitch),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_range(i..i))
        .collect::<Vec<_>>();
        let (ast, errors) = build_control_flow_ast(&mut tokens.iter().peekable());
        println!("AST structure: {ast:#?}");
        let Some(Unit::SwitchBlock { cases, .. }) =
            ast.iter().find(|u| matches!(u, Unit::SwitchBlock { .. }))
        else {
            panic!("AST structure error");
        };
        let Some(case1) = cases
            .iter()
            .find(|c| c.value.content() == &CaseBranchValue::Case(BigUint::from(1u64)))
        else {
            panic!("Case(1) not found");
        };
        println!("Case(1) tokens: {:#?}", case1.units);
        let mut rng = DummyRng;
        let mut ast_iter = ast.clone().into_iter().peekable();
        let (_tokens, _warnings) = parse_control_flow_ast(&mut ast_iter, &mut rng);
        let mut rng = DummyRng;
        let mut ast_iter = ast.into_iter().peekable();
        let (_tokens, _warnings) = parse_control_flow_ast(&mut ast_iter, &mut rng);
        assert_eq!(errors, vec![]);
    }
}
