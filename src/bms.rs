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

use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use fraction::GenericDecimal;
use num::BigUint;

pub mod command;
pub mod error;
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
    error::ParseErrorWithRange,
    lex::{LexOutput, LexWarningWithRange},
    model::Bms,
    parse::{
        check_playing::{PlayingCheckOutput, PlayingError, PlayingWarning},
        token_processor::{TokenProcessor, TokenProcessorResult},
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ParseConfig<T, P, R> {
    key_mapper: PhantomData<fn() -> T>,
    prompter: P,
    rng: R,
    use_minor: bool,
}

#[cfg(feature = "rand")]
pub fn default_config()
-> ParseConfig<KeyLayoutBeat, AlwaysWarnAndUseNewer, RandRng<rand::rngs::StdRng>> {
    use rand::{SeedableRng as _, rngs::StdRng};
    ParseConfig {
        key_mapper: PhantomData,
        prompter: AlwaysWarnAndUseNewer,
        rng: RandRng(StdRng::from_os_rng()),
        use_minor: true,
    }
}

pub fn default_config_with_rng<R>(rng: R) -> ParseConfig<KeyLayoutBeat, AlwaysWarnAndUseNewer, R> {
    ParseConfig {
        key_mapper: PhantomData,
        prompter: AlwaysWarnAndUseNewer,
        rng,
        use_minor: true,
    }
}

impl<T, P, R> ParseConfig<T, P, R> {
    pub fn prompter<P2: Prompter>(self, prompter: P2) -> ParseConfig<T, P2, R> {
        ParseConfig {
            key_mapper: PhantomData,
            prompter,
            rng: self.rng,
            use_minor: self.use_minor,
        }
    }

    pub fn rng<R2: Rng>(self, rng: R2) -> ParseConfig<T, P, R2> {
        ParseConfig {
            key_mapper: PhantomData,
            prompter: self.prompter,
            rng,
            use_minor: self.use_minor,
        }
    }

    pub(crate) fn build(self) -> (impl TokenProcessor<Output = Bms>, P)
    where
        T: KeyLayoutMapper,
        P: Prompter,
        R: Rng,
    {
        struct AggregateTokenProcessor<T, R> {
            key_mapper: PhantomData<fn() -> T>,
            rng: Rc<RefCell<R>>,
            use_minor: bool,
        }
        impl<T: KeyLayoutMapper, R: Rng> TokenProcessor for AggregateTokenProcessor<T, R> {
            type Output = Bms;

            fn process<P: Prompter>(
                &self,
                input: &mut &[&TokenWithRange<'_>],
                prompter: &P,
            ) -> TokenProcessorResult<Self::Output> {
                if self.use_minor {
                    minor_preset::<T, R>(Rc::clone(&self.rng)).process(input, prompter)
                } else {
                    common_preset::<T, R>(Rc::clone(&self.rng)).process(input, prompter)
                }
            }
        }
        (
            AggregateTokenProcessor::<T, R> {
                key_mapper: PhantomData,
                rng: Rc::new(RefCell::new(self.rng)),
                use_minor: self.use_minor,
            },
            self.prompter,
        )
    }
}

/// Parse a BMS file from source text with the specified command preset.
pub fn parse_bms<T: KeyLayoutMapper, P: Prompter, R: Rng>(
    source: &str,
    config: ParseConfig<T, P, R>,
) -> Result<BmsOutput, ParseErrorWithRange> {
    // Parse tokens using default channel parser
    let LexOutput {
        tokens,
        lex_warnings,
    } = lex::TokenStream::parse_lex(source);

    // Convert lex warnings to BmsWarning
    let mut warnings: Vec<BmsWarning> = lex_warnings.into_iter().map(BmsWarning::Lex).collect();

    let (proc, prompter) = config.build();
    let bms = Bms::from_token_stream::<'_, T, _, _>(&tokens, proc, prompter)?;

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
