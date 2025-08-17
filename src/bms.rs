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

use fraction::GenericDecimal;
use num::BigUint;

pub mod ast;
pub mod command;
pub mod lex;
pub mod parse;
pub mod prelude;

use thiserror::Error;

use self::{
    ast::{AstRoot, BmsAstBuildOutput, BmsAstParseOutput, rng::RandRng},
    lex::{BmsLexOutput, LexWarningWithPos, TokenRefStream},
    parse::{
        BmsParseOutput, ParseWarning, ParseWarningWithPos,
        check_playing::{PlayingError, PlayingWarning},
        model::Bms,
    },
};

/// Decimal type used throughout the BMS module.
///
/// This is a type alias for `GenericDecimal<BigUint, usize>` which provides
/// arbitrary precision decimal arithmetic for BMS parsing.
pub type Decimal = GenericDecimal<BigUint, usize>;

/// An error occurred when parsing the BMS format file.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BmsWarning {
    /// An error comes from lexical analyzer.
    #[error("Warn: lex: {0}")]
    Lex(#[from] LexWarningWithPos),
    /// An error comes from syntax parser.
    #[error("Warn: parse: {0}")]
    Parse(#[from] ParseWarningWithPos),
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

    // Parse tokens using default channel parser
    let BmsLexOutput {
        tokens,
        lex_warnings,
    } = lex::TokenStream::parse_lex(source);

    // Convert lex warnings to BmsWarning
    let mut warnings: Vec<BmsWarning> = lex_warnings.into_iter().map(BmsWarning::Lex).collect();

    // Parse BMS using default RNG and prompt handler
    let rng = RandRng(StdRng::from_os_rng());
    // Build AST
    let BmsAstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    warnings.extend(
        ast_build_warnings
            .into_iter()
            .map(|w| w.map(ParseWarning::AstBuild))
            .map(BmsWarning::Parse),
    );
    // Parse AST
    let BmsAstParseOutput { token_refs: tokens } = TokenRefStream::from_ast_root(root, rng);
    // According to [BMS command memo#BEHAVIOR IN GENERAL IMPLEMENTATION](https://hitkey.bms.ms/cmds.htm#BEHAVIOR-IN-GENERAL-IMPLEMENTATION), the newer values are used for the duplicated objects.
    let BmsParseOutput {
        bms,
        parse_warnings,
        playing_warnings,
        playing_errors,
    } = Bms::from_token_stream(&tokens, parse::prompt::AlwaysWarnAndUseNewer);

    // Convert parse warnings to BmsWarning
    warnings.extend(parse_warnings.into_iter().map(BmsWarning::Parse));

    // Convert playing warnings to BmsWarning
    warnings.extend(playing_warnings.into_iter().map(BmsWarning::PlayingWarning));

    // Convert playing errors to BmsWarning
    warnings.extend(playing_errors.into_iter().map(BmsWarning::PlayingError));

    BmsOutput { bms, warnings }
}
