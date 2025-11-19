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

pub mod command;

pub mod lex;
pub mod model;
pub mod parse;
pub mod prelude;
pub mod rng;
pub mod unparse;

use thiserror::Error;

#[cfg(feature = "diagnostics")]
use ariadne::Report;

#[cfg(feature = "diagnostics")]
use crate::diagnostics::{SimpleSource, ToAriadne};

use self::{
    lex::{LexOutput, LexWarningWithRange},
    model::Bms,
    parse::{
        ParseErrorWithRange, ParseWarningWithRange,
        check_playing::{PlayingCheckOutput, PlayingError, PlayingWarning},
        token_processor::{TokenProcessor, full_preset, rewrite_relaxed_tokens},
    },
    prelude::*,
};

/// Decimal type used throughout the BMS module.
///
/// This is a type alias for `GenericDecimal<BigUint, usize>` which provides
/// arbitrary precision decimal arithmetic for BMS parsing.
pub type Decimal = fraction::BigDecimal;

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

/// A configuration builder for [`parse_bms`]. Its methods can be chained to set parameters you want.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[must_use]
pub struct ParseConfig<T, P, R> {
    key_mapper: PhantomData<fn() -> T>,
    prompter: P,
    rng: R,
    use_relaxed: bool,
}

/// Creates the default configuration builder with the basic key layout [`KeyLayoutBeat`], the prompter [`AlwaysWarnAndUseNewer`] and the standard RNG [`rand::rngs::StdRng`].
#[cfg(feature = "rand")]
pub fn default_config()
-> ParseConfig<KeyLayoutBeat, AlwaysWarnAndUseNewer, RandRng<rand::rngs::StdRng>> {
    use rand::{SeedableRng as _, rngs::StdRng};
    ParseConfig {
        key_mapper: PhantomData,
        prompter: AlwaysWarnAndUseNewer,
        rng: RandRng(StdRng::from_os_rng()),
        use_relaxed: true,
    }
}

/// Creates the default configuration builder with the basic key layout [`KeyLayoutBeat`], the prompter [`AlwaysWarnAndUseNewer`] and the Java-compatible RNG.
/// This version is available when the `rand` feature is not enabled.
#[cfg(not(feature = "rand"))]
pub fn default_config() -> ParseConfig<KeyLayoutBeat, AlwaysWarnAndUseNewer, rng::JavaRandom> {
    ParseConfig {
        key_mapper: PhantomData,
        prompter: AlwaysWarnAndUseNewer,
        rng: rng::JavaRandom::default(),
        use_relaxed: true,
    }
}

/// Creates the default configuration builder with the basic key layout [`KeyLayoutBeat`], the prompter [`AlwaysWarnAndUseNewer`] and your RNG.
pub fn default_config_with_rng<R>(rng: R) -> ParseConfig<KeyLayoutBeat, AlwaysWarnAndUseNewer, R> {
    ParseConfig {
        key_mapper: PhantomData,
        prompter: AlwaysWarnAndUseNewer,
        rng,
        use_relaxed: true,
    }
}

impl<T, P, R> ParseConfig<T, P, R> {
    /// Sets the key mapper to the `T2` one.
    pub fn key_mapper<T2: KeyLayoutMapper>(self) -> ParseConfig<T2, P, R> {
        ParseConfig {
            key_mapper: PhantomData,
            prompter: self.prompter,
            rng: self.rng,
            use_relaxed: self.use_relaxed,
        }
    }

    /// Sets the prompter to `prompter`.
    pub fn prompter<P2: Prompter>(self, prompter: P2) -> ParseConfig<T, P2, R> {
        ParseConfig {
            key_mapper: PhantomData,
            prompter,
            rng: self.rng,
            use_relaxed: self.use_relaxed,
        }
    }

    /// Sets the RNG to `rng`.
    pub fn rng<R2: Rng>(self, rng: R2) -> ParseConfig<T, P, R2> {
        ParseConfig {
            key_mapper: PhantomData,
            prompter: self.prompter,
            rng,
            use_relaxed: self.use_relaxed,
        }
    }

    /// Change to use pedantic token processors that don't recognize common mistakes.
    pub fn use_pedantic(self) -> Self {
        Self {
            use_relaxed: false,
            ..self
        }
    }

    /// Change to use relaxed token processors that recognize common mistakes. This is the default option.
    pub fn use_relaxed(self) -> Self {
        Self {
            use_relaxed: true,
            ..self
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
        }
        impl<T: KeyLayoutMapper, R: Rng> TokenProcessor for AggregateTokenProcessor<T, R> {
            type Output = Bms;

            fn process<'a, 't, P: Prompter>(
                &self,
                ctx: &mut parse::token_processor::ProcessContext<'a, 't, P>,
            ) -> Result<Self::Output, ParseErrorWithRange> {
                full_preset::<T, R>(Rc::clone(&self.rng)).process(ctx)
            }
        }
        (
            AggregateTokenProcessor::<T, R> {
                key_mapper: PhantomData,
                rng: Rc::new(RefCell::new(self.rng)),
            },
            self.prompter,
        )
    }
}

/// Parse a BMS file from source text with the specified command preset.
pub fn parse_bms<T: KeyLayoutMapper, P: Prompter, R: Rng>(
    source: &str,
    config: ParseConfig<T, P, R>,
) -> BmsOutput {
    // Parse tokens using default channel parser
    let LexOutput {
        mut tokens,
        lex_warnings,
    } = lex::TokenStream::parse_lex(source);

    // Convert lex warnings to BmsWarning
    let mut warnings: Vec<BmsWarning> = lex_warnings.into_iter().map(BmsWarning::Lex).collect();

    if config.use_relaxed {
        rewrite_relaxed_tokens(&mut tokens);
    }
    let parse_output = Bms::from_token_stream::<'_, T, _, _>(&tokens, config);
    let bms_result = parse_output.bms;
    // Convert parse warnings to BmsWarning
    warnings.extend(
        parse_output
            .parse_warnings
            .into_iter()
            .map(BmsWarning::Parse),
    );

    if let Ok(ref bms) = bms_result {
        let PlayingCheckOutput {
            playing_warnings,
            playing_errors,
        } = bms.check_playing::<T>();

        // Convert playing warnings to BmsWarning
        warnings.extend(playing_warnings.into_iter().map(BmsWarning::PlayingWarning));

        // Convert playing errors to BmsWarning
        warnings.extend(playing_errors.into_iter().map(BmsWarning::PlayingError));
    }

    BmsOutput {
        bms: bms_result,
        warnings,
    }
}

/// Output of parsing a BMS file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[must_use]
pub struct BmsOutput {
    /// The parsed BMS data.
    pub bms: Result<Bms, ParseErrorWithRange>,
    /// Warnings that occurred during parsing.
    pub warnings: Vec<BmsWarning>,
}

#[cfg(feature = "diagnostics")]
impl ToAriadne for BmsWarning {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        use BmsWarning::*;
        match self {
            Lex(e) => e.to_report(src),
            Parse(e) => e.to_report(src),
            PlayingWarning(w) => w.to_report(src),
            PlayingError(e) => e.to_report(src),
        }
    }
}
