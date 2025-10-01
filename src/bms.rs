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
pub mod model;
pub mod parse;
pub mod prelude;
pub mod unparse;

use thiserror::Error;

use ariadne::{Label, Report, ReportKind};

use crate::diagnostics::{SimpleSource, ToAriadne};

#[cfg(feature = "rand")]
use self::ast::rng::RandRng;
use self::{
    ast::{
        AstBuildOutput, AstBuildWarningWithRange, AstParseOutput, AstParseWarningWithRange,
        AstRoot, rng::Rng,
    },
    command::channel::mapper::KeyLayoutMapper,
    lex::{LexOutput, LexWarningWithRange},
    model::Bms,
    parse::{
        ParseOutput, ParseWarningWithRange,
        check_playing::{PlayingCheckOutput, PlayingError, PlayingWarning},
    },
    prelude::AlwaysWarnAndUseNewer,
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
    Lex(#[from] LexWarningWithRange),
    /// An error comes from AST builder.
    #[error("Warn: ast_build: {0}")]
    AstBuild(#[from] AstBuildWarningWithRange),
    /// A warning comes from AST parsing.
    #[error("Warn: ast_parse: {0}")]
    AstParse(#[from] AstParseWarningWithRange),
    /// An error comes from syntax parser.
    #[error("Warn: parse: {0}")]
    Parse(#[from] ParseWarningWithRange),
    /// A warning for playing.
    #[error("Warn: playing: {0}")]
    PlayingWarning(#[from] PlayingWarning),
    /// An error for playing.
    #[error("Error: playing: {0}")]
    PlayingError(#[from] PlayingError),
}

/// Output of parsing a BMS file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use]
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
/// use bms_rs::bms::command::channel::mapper::KeyLayoutBeat;
///
/// let source = "#TITLE Test Song\n#BPM 120\n#00101:0101";
/// let BmsOutput { bms, warnings }: BmsOutput = parse_bms::<KeyLayoutBeat>(source);
/// println!("Title: {}", bms.header.title.as_deref().unwrap_or("Unknown"));
/// println!("BPM: {}", bms.arrangers.bpm.unwrap_or(120.into()));
/// println!("Warnings: {:?}", warnings);
/// ```
#[cfg(feature = "rand")]
pub fn parse_bms<T: KeyLayoutMapper>(source: &str) -> BmsOutput {
    use rand::{SeedableRng, rngs::StdRng};

    // Parse BMS using default RNG and prompt handler
    let rng = RandRng(StdRng::from_os_rng());
    parse_bms_with_rng::<T, _>(source, rng)
}

/// Parse a BMS file from source text using a custom random number generator.
///
/// This function provides a convenient way to parse a BMS file in one step.
/// It uses the default channel parser and a custom random number generator.
pub fn parse_bms_with_rng<T: KeyLayoutMapper, R: Rng>(source: &str, rng: R) -> BmsOutput {
    // Parse tokens using default channel parser
    let LexOutput {
        tokens,
        lex_warnings,
    } = lex::TokenStream::parse_lex(source);

    // Convert lex warnings to BmsWarning
    let mut warnings: Vec<BmsWarning> = lex_warnings.into_iter().map(BmsWarning::Lex).collect();
    // Build AST
    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    warnings.extend(ast_build_warnings.into_iter().map(BmsWarning::AstBuild));

    // Parse AST
    let (AstParseOutput { token_refs }, ast_parse_warnings) = root.parse_with_warnings(rng);
    // According to [BMS command memo#BEHAVIOR IN GENERAL IMPLEMENTATION](https://hitkey.bms.ms/cmds.htm#BEHAVIOR-IN-GENERAL-IMPLEMENTATION), the newer values are used for the duplicated objects.
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream::<'_, T, AlwaysWarnAndUseNewer>(token_refs, AlwaysWarnAndUseNewer);

    // Convert ast-parse and parse warnings to BmsWarning
    warnings.extend(ast_parse_warnings.into_iter().map(BmsWarning::AstParse));
    warnings.extend(parse_warnings.into_iter().map(BmsWarning::Parse));

    let PlayingCheckOutput {
        playing_warnings,
        playing_errors,
    } = bms.check_playing::<T>();

    // Convert playing warnings to BmsWarning
    warnings.extend(playing_warnings.into_iter().map(BmsWarning::PlayingWarning));

    // Convert playing errors to BmsWarning
    warnings.extend(playing_errors.into_iter().map(BmsWarning::PlayingError));

    BmsOutput { bms, warnings }
}

impl ToAriadne for BmsWarning {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        use BmsWarning::*;
        match self {
            Lex(e) => e.to_report(src),
            AstBuild(e) => e.to_report(src),
            AstParse(e) => e.to_report(src),
            Parse(e) => e.to_report(src),
            // PlayingWarning / PlayingError have no position, locate to file start 0..0
            PlayingWarning(w) => {
                let filename = src.name().to_string();
                Report::build(ReportKind::Warning, (filename.clone(), 0..0))
                    .with_message(format!("playing warning: {w}"))
                    .with_label(Label::new((filename, 0..0)))
                    .finish()
            }
            PlayingError(e) => {
                let filename = src.name().to_string();
                Report::build(ReportKind::Error, (filename.clone(), 0..0))
                    .with_message(format!("playing error: {e}"))
                    .with_label(Label::new((filename, 0..0)))
                    .finish()
            }
        }
    }
}
