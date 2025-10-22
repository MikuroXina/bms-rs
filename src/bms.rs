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

use std::{cell::RefCell, rc::Rc};

use fraction::GenericDecimal;
use num::BigUint;

pub mod command;
pub mod lex;
pub mod model;
pub mod parse;
pub mod prelude;
pub mod rng;
pub mod unparse;

use thiserror::Error;

use ariadne::{Label, Report, ReportKind};

use crate::diagnostics::{SimpleSource, ToAriadne};

use self::{
    lex::{LexOutput, LexWarningWithRange},
    model::Bms,
    parse::{
        ParseErrorWithRange, ParseOutput, ParseWarningWithRange,
        check_playing::{PlayingCheckOutput, PlayingError, PlayingWarning},
        token_processor::{self, TokenProcessor},
    },
    prelude::*,
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

/// The token processor used in [`parse_bms`]. It picks newer token on duplicated, uses a default random number generator (from [`rand::rngs::OsRng`]), and parses tokens as possible.
#[cfg(feature = "rand")]
pub fn default_preset(bms: Rc<RefCell<Bms>>) -> impl TokenProcessor {
    use rand::{SeedableRng as _, rngs::StdRng};
    token_processor::minor_preset::<_, KeyLayoutBeat, _>(
        bms,
        &AlwaysWarnAndUseNewer,
        Rc::new(RefCell::new(RandRng(StdRng::from_os_rng()))),
    )
}

/// The token processor with your custom random number generator (extended from [`default_preset`]). It picks newer token on duplicated, and parses tokens as possible.
pub fn default_preset_with_rng<R: Rng + 'static>(
    rng: R,
) -> impl FnOnce(Rc<RefCell<Bms>>) -> Box<dyn TokenProcessor> {
    move |bms| {
        Box::new(token_processor::minor_preset::<_, KeyLayoutBeat, _>(
            bms,
            &AlwaysWarnAndUseNewer,
            Rc::new(RefCell::new(rng)),
        ))
    }
}

/// The token processor with your custom prompter (extended from [`default_preset`]). It uses a default random number generator (from [`rand::rngs::OsRng`]), and parses tokens as possible.
#[cfg(feature = "rand")]
pub fn default_preset_with_prompter<'a, P: Prompter + 'a>(
    prompter: &'a P,
) -> impl FnOnce(Rc<RefCell<Bms>>) -> Box<dyn TokenProcessor + 'a> {
    use rand::{SeedableRng as _, rngs::StdRng};
    move |bms| {
        Box::new(token_processor::minor_preset::<_, KeyLayoutBeat, _>(
            bms,
            prompter,
            Rc::new(RefCell::new(RandRng(StdRng::from_os_rng()))),
        ))
    }
}

/// Parse a BMS file from source text.
///
/// This function provides a convenient way to parse a BMS file in one step.
/// It uses the default channel parser and a default random number generator (from [`rand::rngs::OsRng`]). See also [`default_preset`].
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
pub fn parse_bms<T: KeyLayoutMapper>(source: &str) -> Result<BmsOutput, ParseErrorWithRange> {
    parse_bms_with_preset::<T, _, _>(source, default_preset)
}

/// Parse a BMS file from source text with the specified command preset.
pub fn parse_bms_with_preset<
    T: KeyLayoutMapper,
    F: FnOnce(Rc<RefCell<Bms>>) -> TP,
    TP: TokenProcessor,
>(
    source: &str,
    preset: F,
) -> Result<BmsOutput, ParseErrorWithRange> {
    // Parse tokens using default channel parser
    let LexOutput {
        tokens,
        lex_warnings,
    } = lex::TokenStream::parse_lex(source);

    // Convert lex warnings to BmsWarning
    let mut warnings: Vec<BmsWarning> = lex_warnings.into_iter().map(BmsWarning::Lex).collect();

    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream::<'_, T, TP, _>(&tokens, preset)?;

    warnings.extend(parse_warnings.into_iter().map(BmsWarning::Parse));

    let PlayingCheckOutput {
        playing_warnings,
        playing_errors,
    } = bms.check_playing::<T>();

    // Convert playing warnings to BmsWarning
    warnings.extend(playing_warnings.into_iter().map(BmsWarning::PlayingWarning));

    // Convert playing errors to BmsWarning
    warnings.extend(playing_errors.into_iter().map(BmsWarning::PlayingError));

    Ok(BmsOutput { bms, warnings })
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

impl ToAriadne for BmsWarning {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        use BmsWarning::*;
        match self {
            Lex(e) => e.to_report(src),
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
