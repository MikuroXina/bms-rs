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

use rng::Rng;
use thiserror::Error;

use crate::bms::{
    command::mixin::SourcePosMixin,
    lex::{TokenRefStream, TokenStream, token::TokenWithPos},
};

use self::{
    ast_build::{Unit, build_control_flow_ast},
    ast_parse::parse_control_flow_ast,
};

/// The root of the AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstRoot<'a> {
    /// The units of the AST.
    pub units: Vec<Unit<'a>>,
}

/// The output of building the AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstBuildOutput<'a> {
    /// The units of the AST.
    pub root: AstRoot<'a>,
    /// The errors that occurred during building.
    pub ast_build_warnings: Vec<AstBuildWarningWithPos>,
}

impl<'a> AstRoot<'a> {
    /// Builds the AST from a token stream.
    pub fn from_token_stream(
        token_stream: impl IntoIterator<Item = &'a TokenWithPos<'a>>,
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
pub struct AstParseOutput<'a> {
    /// The tokens that were parsed.
    pub token_refs: TokenRefStream<'a>,
    /// The warnings that occurred during parsing.
    pub ast_parse_warnings: Vec<AstParseWarningWithPos>,
}

impl<'a> AstRoot<'a> {
    /// Parses the AST using a random number generator.
    pub fn parse(self, mut rng: impl Rng) -> AstParseOutput<'a> {
        let mut ast_iter = self.units.into_iter().peekable();
        let (tokens, warnings) = parse_control_flow_ast(&mut ast_iter, &mut rng);
        AstParseOutput {
            token_refs: TokenRefStream { token_refs: tokens },
            ast_parse_warnings: warnings,
        }
    }
}

impl<'a> AstRoot<'a> {
    /// Extracts all tokens from the AST and returns them as a TokenStream.
    /// This function flattens the AST structure and returns ALL tokens contained in the AST,
    /// including all branches in Random and Switch blocks. This serves as the inverse of
    /// [`AstRoot::from_token_stream`].
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
    /// Tokens were found between `#RANDOM` and `#IF` that should not be there.
    #[error("unmatched token in random block, e.g. Tokens between Random and If.")]
    UnmatchedTokenInRandomBlock,
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
pub type AstBuildWarningWithPos = SourcePosMixin<AstBuildWarning>;

/// Control flow parsing errors and warnings during AST execution.
///
/// This enum defines all possible errors that can occur during BMS control flow AST execution.
/// Each variant represents a specific type of execution error or unexpected behavior.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AstParseWarning {
    /// The RNG generated a value that is out of range for a Random block.
    /// This can happen when the RNG implementation returns values outside the expected range.
    #[error("RNG generated value out of range for random block")]
    RandomValueOutOfRange {
        /// The expected range of values for the random block.
        expected_range: std::ops::RangeInclusive<num::BigUint>,
        /// The actual value generated by the RNG.
        actual_value: num::BigUint,
    },
    /// The RNG generated a value that is out of range for a Switch block.
    /// This can happen when the RNG implementation returns values outside the expected range.
    #[error("RNG generated value out of range for switch block")]
    SwitchValueOutOfRange {
        /// The expected range of values for the switch block.
        expected_range: std::ops::RangeInclusive<num::BigUint>,
        /// The actual value generated by the RNG.
        actual_value: num::BigUint,
    },
}

/// A [`AstParseWarning`] type with position information.
pub type AstParseWarningWithPos = SourcePosMixin<AstParseWarning>;

#[cfg(test)]
mod tests {
    use core::ops::RangeInclusive;

    use num::BigUint;

    use super::*;
    use crate::bms::{
        ast::ast_build::{CaseBranch, CaseBranchValue, Unit},
        command::mixin::SourcePosMixinExt,
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
        use Token::*;
        let tokens = vec![
            Title("11000000"),
            Switch(BigUint::from(2u32)),
            Case(BigUint::from(1u32)),
            Title("00220000"),
            Random(BigUint::from(2u32)),
            If(BigUint::from(1u32)),
            Title("00550000"),
            ElseIf(BigUint::from(2u32)),
            Title("00006600"),
            EndIf,
            EndRandom,
            Skip,
            Case(BigUint::from(2u32)),
            Title("00003300"),
            Skip,
            EndSwitch,
            Title("00000044"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_manual(i, i))
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
            .find(|c| c.value == CaseBranchValue::Case(BigUint::from(1u64)))
        else {
            panic!("Case(1) not found");
        };
        println!("Case(1) tokens: {:#?}", case1.tokens);
        assert_eq!(errors, vec![]);
        assert!(matches!(&ast[0], Unit::TokenWithPos(_))); // 11000000
        assert!(matches!(&ast[1], Unit::SwitchBlock { .. }));
        assert!(matches!(&ast[2], Unit::TokenWithPos(_))); // 00000044
        let Unit::SwitchBlock { cases, .. } = &ast[1] else {
            panic!("AST structure error");
        };
        let Some(CaseBranch { tokens, .. }) = cases
            .iter()
            .find(|c| c.value == CaseBranchValue::Case(BigUint::from(1u64)))
        else {
            panic!("Case(1) not found");
        };
        assert!(matches!(&tokens[0], Unit::TokenWithPos(_))); // 00220000
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
                .tokens
                .iter()
                .filter_map(|u| match u {
                    Unit::TokenWithPos(token) => Some(token),
                    _ => None,
                })
                .any(|u| matches!(u.content(), Title("00550000")))
        );
        assert!(
            if_block
                .branches
                .get(&BigUint::from(2u64))
                .unwrap()
                .tokens
                .iter()
                .filter_map(|u| match u {
                    Unit::TokenWithPos(token) => Some(token),
                    _ => None,
                })
                .any(|u| matches!(u.content(), Title("00006600")))
        );
        let Some(CaseBranch { tokens, .. }) = cases
            .iter()
            .find(|c| c.value == CaseBranchValue::Case(BigUint::from(2u64)))
        else {
            panic!("Case(2) not found");
        };
        assert!({
            let Unit::TokenWithPos(token) = &tokens[0] else {
                panic!("Unit::TokenWithPos expected, got: {tokens:?}");
            };
            matches!(token.content(), Title("00003300"))
        });
        let mut rng = DummyRng;
        let mut ast_iter = ast.into_iter().peekable();
        let (tokens, _warnings) = parse_control_flow_ast(&mut ast_iter, &mut rng);
        let expected = ["11000000", "00003300", "00000044"];
        assert_eq!(tokens.len(), 3);
        for (i, t) in tokens.iter().enumerate() {
            assert_eq!(t.content(), &Title(expected[i]));
        }
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn test_switch_insane_tokenized() {
        use Token::*;
        let tokens = vec![
            Switch(BigUint::from(5u32)),
            Def,
            Title("0055"),
            Skip,
            Case(BigUint::from(1u32)),
            Title("0100000000000000"),
            Random(BigUint::from(2u32)),
            If(BigUint::from(1u32)),
            Title("04"),
            Else,
            Title("05"),
            EndIf,
            // Missing EndRandom!!!
            Case(BigUint::from(2u32)),
            Title("0200000000000000"),
            Skip,
            Case(BigUint::from(3u32)),
            Title("0300000000000000"),
            Switch(BigUint::from(2u32)),
            Case(BigUint::from(1u32)),
            Title("1111"),
            Skip,
            Case(BigUint::from(2u32)),
            Title("2222"),
            Skip,
            EndSwitch,
            Skip,
            EndSwitch,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.into_wrapper_manual(i, i))
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
            .find(|c| c.value == CaseBranchValue::Case(BigUint::from(1u64)))
        else {
            panic!("Case(1) not found");
        };
        println!("Case(1) tokens: {:#?}", case1.tokens);
        let mut rng = DummyRng;
        let mut ast_iter = ast.clone().into_iter().peekable();
        let (_tokens, _warnings) = parse_control_flow_ast(&mut ast_iter, &mut rng);
        let mut rng = DummyRng;
        let mut ast_iter = ast.into_iter().peekable();
        let (_tokens, _warnings) = parse_control_flow_ast(&mut ast_iter, &mut rng);
        assert_eq!(errors, vec![]);
    }
}
