//! The parser module of BMS(.bms/.bme/.bml/.pms) file.
//!
//! This module consists of two phases: lexical analyzing and token parsing.
//!
//! `lex` module provides definitions of BMS tokens and a translator from string into them. It supports major commands as possible, because the BMS specification is not standardized yet. If you found a lack of definition,  please tell me by opening an issue (only if not open yet).
//!
//! `parse` module provides definitions of BMS semantic objects and managers of BMS score data. The notes are serializable, but parsed result can't bring back into the BMS format text because of there are randomized syntax in BMS.
//!
//! `time` module provides definition of timing for notes as [`time::Track`] and [`time::ObjTime`].
//!
//! In detail, our policies are:
//!
//! - Support only UTF-8 (as required `String` to input).
//! - Do not support editing BMS source text.
//! - Do not support commands having ambiguous semantics.
//! - Do not support syntax came from typo (such as `#RONDOM` or `#END IF`).

use std::ops::{Deref, DerefMut};

use fraction::GenericDecimal;
use num::BigUint;

pub mod ast;
pub mod command;
pub mod lex;
pub mod parse;
pub mod prelude;

use thiserror::Error;

use crate::bms::{
    ast::{
        BmsAstBuildOutput, BmsAstParseOutput,
        rng::{RandRng, Rng},
        structure::AstRoot,
    },
    command::PositionWrapper,
    lex::token::Token,
    parse::model::Bms,
    prelude::{AlwaysWarnAndUseOlder, PromptHandler},
};

use self::{
    ast::structure::{AstBuildWarning, AstParseWarning},
    lex::BmsLexOutput,
    parse::{
        BmsParseOutput, ParseWarning,
        check_playing::{PlayingCheckOutput, PlayingError, PlayingWarning},
    },
};

/// Decimal type used throughout the BMS module.
///
/// This is a type alias for `GenericDecimal<BigUint, usize>` which provides
/// arbitrary precision decimal arithmetic for BMS parsing.
pub type Decimal = GenericDecimal<BigUint, usize>;

/// The type of parsing tokens iter.
pub struct BmsTokenIter<'a>(std::iter::Peekable<std::slice::Iter<'a, PositionWrapper<Token<'a>>>>);

impl<'a> BmsTokenIter<'a> {
    /// Create iter from BmsLexOutput reference.
    pub fn from_lex_output(value: &'a BmsLexOutput) -> Self {
        Self(value.tokens.iter().as_slice().iter().peekable())
    }
    /// Create iter from Token list reference.
    pub fn from_tokens(value: &'a [PositionWrapper<Token<'a>>]) -> Self {
        Self(value.iter().peekable())
    }
}

impl<'a, T: AsRef<[PositionWrapper<Token<'a>>]> + ?Sized> From<&'a T> for BmsTokenIter<'a> {
    fn from(value: &'a T) -> Self {
        Self(value.as_ref().iter().peekable())
    }
}

impl<'a> Deref for BmsTokenIter<'a> {
    type Target = std::iter::Peekable<std::slice::Iter<'a, PositionWrapper<Token<'a>>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for BmsTokenIter<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
/// An error occurred when parsing the BMS format file.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BmsWarning {
    /// An error comes from lexical analyzer.
    #[error("Warn: lex: {0}")]
    LexWarning(#[from] PositionWrapper<lex::LexWarning>),
    /// Violation of control flow rule.
    #[error("Warn: AST build: {0}")]
    AstBuildWarning(#[from] PositionWrapper<AstBuildWarning>),
    /// Violation detected during AST execution.
    #[error("Warn: AST parse: {0}")]
    AstParseWarning(#[from] PositionWrapper<AstParseWarning>),
    /// An error comes from syntax parser.
    #[error("Warn: parse: {0}")]
    ParseWarning(#[from] PositionWrapper<ParseWarning>),
    /// A warning for playing.
    #[error("Warn: playing: {0}")]
    PlayingWarning(#[from] PlayingWarning),
    /// An error for playing.
    #[error("Error: playing: {0}")]
    PlayingError(#[from] PlayingError),
}

/// Output of parsing a BMS file.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BmsOutput {
    /// The parsed BMS data.
    pub bms: Bms,
    /// Warnings that occurred during parsing.
    pub warnings: Vec<BmsWarning>,
}

/// Parse a BMS file from source text.
///
/// This function provides a convenient way to parse a BMS file in one step.
/// It uses the default channel parser and a default random number generator (from [`rand::rngs::OsRng`]).
///
/// # Example
///
/// ```
/// use bms_rs::bms::{parse_bms, BmsOutput};
///
/// let source = "#TITLE Test Song\n#BPM 120\n#00101:0101";
/// let BmsOutput { bms, warnings } = parse_bms(source);
/// println!("Title: {}", bms.header.title.as_deref().unwrap_or("Unknown"));
/// println!("BPM: {}", bms.arrangers.bpm.unwrap_or(120.into()));
/// println!("Warnings: {:?}", warnings);
/// ```
pub fn parse_bms(source: &str) -> BmsOutput {
    use rand::{SeedableRng, rngs::StdRng};
    let rng = RandRng(StdRng::from_os_rng());
    parse_bms_with_rng(source, rng)
}

/// Parse bms file with rng.
///
/// A step of [`parse_bms`]
pub fn parse_bms_with_rng(source: &str, rng: impl Rng) -> BmsOutput {
    // Parse tokens using default channel parser
    let BmsLexOutput {
        tokens,
        lex_warnings,
    } = parse_bms_step_lex(source);

    let BmsOutput {
        bms,
        warnings: after_warnings,
    } = parse_bms_with_tokens(&tokens, rng);

    // Convert lex warnings to BmsWarning
    let mut warnings: Vec<BmsWarning> = lex_warnings
        .into_iter()
        .map(BmsWarning::LexWarning)
        .collect();

    warnings.extend(after_warnings);

    BmsOutput { bms, warnings }
}

/// Parse bms file with tokens and rng.
///
/// A step of [`parse_bms`]
pub fn parse_bms_with_tokens(tokens: &[PositionWrapper<Token<'_>>], rng: impl Rng) -> BmsOutput {
    // Parse BMS using default RNG and prompt handler
    let BmsAstBuildOutput {
        root,
        ast_build_warnings,
    } = parse_bms_step_build_ast(tokens);

    let BmsOutput {
        bms,
        warnings: after_warnings,
    } = parse_bms_with_ast(root, rng);

    // Convert lex warnings to BmsWarning
    let mut warnings: Vec<BmsWarning> = ast_build_warnings
        .into_iter()
        .map(BmsWarning::AstBuildWarning)
        .collect();

    warnings.extend(after_warnings);

    BmsOutput { bms, warnings }
}

/// Parse bms file with [`AstRoot`] and rng.
///
/// A step of [`parse_bms`]
pub fn parse_bms_with_ast(root: AstRoot<'_>, rng: impl Rng) -> BmsOutput {
    // Parse Ast
    let BmsAstParseOutput {
        tokens: tokens_from_ast,
        ast_parse_warnings,
    } = parse_bms_step_parse_ast(root, rng);

    // Convert lex warnings to BmsWarning
    let mut warnings: Vec<BmsWarning> = ast_parse_warnings
        .into_iter()
        .map(BmsWarning::AstParseWarning)
        .collect();

    let BmsOutput {
        bms,
        warnings: after_warnings,
    } = parse_bms_with_tokens_and_prompt_handler(&tokens_from_ast, AlwaysWarnAndUseOlder);

    warnings.extend(after_warnings);

    BmsOutput { bms, warnings }
}

/// Parse bms file with tokens and prompt handler.
///
/// A step of [`parse_bms_with_ast`]
pub fn parse_bms_with_tokens_and_prompt_handler(
    tokens: &[PositionWrapper<Token<'_>>],
    prompt_handler: impl PromptHandler,
) -> BmsOutput {
    // Parse Bms File
    let BmsParseOutput {
        bms,
        parse_warnings,
    } = parse_bms_step_model(tokens, prompt_handler);

    let mut warnings: Vec<BmsWarning> = parse_warnings
        .into_iter()
        .map(BmsWarning::ParseWarning)
        .collect();

    // Check playing
    let PlayingCheckOutput {
        playing_warnings,
        playing_errors,
    } = bms.check_playing();

    warnings.extend(playing_warnings.into_iter().map(BmsWarning::PlayingWarning));

    warnings.extend(playing_errors.into_iter().map(BmsWarning::PlayingError));

    BmsOutput { bms, warnings }
}

/*
* Steps for parsing BMS file
*/

/// Public step function for lexing used by bms.rs and external callers.
///
/// Step 1 of [`parse_bms`]
pub fn parse_bms_step_lex<'a>(source: &'a str) -> BmsLexOutput<'a> {
    lex::parse_lex_tokens(source)
}

/// Public step function for building AST used by `bms.rs` and external callers.
///
/// Step 2 of [`parse_bms`]
pub fn parse_bms_step_build_ast<'a>(
    token_stream: impl Into<BmsTokenIter<'a>>,
) -> BmsAstBuildOutput<'a> {
    ast::build_ast(token_stream)
}

/// Public step function for parsing AST used by `bms.rs` and external callers.
///
/// Step 3 of [`parse_bms`]
pub fn parse_bms_step_parse_ast<'a>(root: AstRoot<'a>, rng: impl Rng) -> BmsAstParseOutput<'a> {
    ast::parse_ast(root, rng)
}

/// Public step function for parsing BMS model used by `bms.rs` and external callers.
///
/// Step 4 of [`parse_bms`]
pub fn parse_bms_step_model<'a>(
    token_iter: impl Into<BmsTokenIter<'a>>,
    prompt_handler: impl PromptHandler,
) -> BmsParseOutput {
    Bms::from_token_stream(token_iter, prompt_handler)
}

/// Public step function for checking playing used by `bms.rs` and external callers.
///
/// Step 5 of [`parse_bms`]
pub fn parse_bms_step_check_playing(bms: &Bms) -> PlayingCheckOutput {
    bms.check_playing()
}
