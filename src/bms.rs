//! The parser module of BMS(.bms/.bme/.bml/.pms) file.
//!
//! This module consists of two phases: lexical analyzing and token parsing.
//!
//! `lex` module provides definitions of BMS tokens and a translator from string into them. It supports major commands as possible, because the BMS specification is not standardized yet. If you found a lack of definition,  please tell me by opening an issue (only if not open yet).
//!
//! `parse` module provides definitions of BMS semantic objects and managers of BMS score data. The notes are serializable, but parsed result can't bring back into the BMS format text because of there are randomized syntax in BMS.
//!
//! `time` module provides definition of timing for notes as [`command::time::Track`] and [`command::time::ObjTime`].
//!
//! In detail, our policies are:
//!
//! - Support only UTF-8 (as required `String` to input).
//! - Do not support editing BMS source text.
//! - Do not support commands having ambiguous semantics.
//! - Do not support syntax came from typo (such as `#RONDOM` or `#END IF`).

use std::{cell::RefCell, rc::Rc};

pub mod command;

pub mod lex;
pub mod model;
pub mod parse;
pub mod prelude;
pub mod process;
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
        token_processor::{
            DefaultTokenRelaxer, NoopTokenModifier, SequentialTokenModifier, TokenModifier,
            TokenProcessor, full_preset,
        },
    },
    prelude::*,
};

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
}

/// A configuration builder for [`parse_bms`]. Its methods can be chained to set parameters you want.
#[must_use]
pub struct ParseConfig<P, R, M> {
    prompter: P,
    rng: R,
    token_modifier: M,
}

/// Creates the default configuration builder with the basic key layout [`BmsLayoutBeat`], the prompter [`AlwaysWarnAndUseNewer`] and the standard RNG [`rand::rngs::StdRng`].
#[cfg(feature = "rand")]
pub fn default_config()
-> ParseConfig<AlwaysWarnAndUseNewer, RandRng<rand::rngs::StdRng>, DefaultTokenRelaxer> {
    ParseConfig {
        prompter: AlwaysWarnAndUseNewer,
        rng: RandRng(rand::make_rng()),
        token_modifier: DefaultTokenRelaxer,
    }
}

/// Creates the default configuration builder with the basic key layout [`BmsLayoutBeat`], the prompter [`AlwaysWarnAndUseNewer`] and the Java-compatible RNG.
/// This version is available when the `rand` feature is not enabled.
#[cfg(not(feature = "rand"))]
pub fn default_config() -> ParseConfig<AlwaysWarnAndUseNewer, rng::JavaRandom, DefaultTokenRelaxer>
{
    ParseConfig {
        prompter: AlwaysWarnAndUseNewer,
        rng: rng::JavaRandom::default(),
        token_modifier: DefaultTokenRelaxer,
    }
}

/// Creates the default configuration builder with the basic key layout [`BmsLayoutBeat`], the prompter [`AlwaysWarnAndUseNewer`] and your RNG.
pub const fn default_config_with_rng<R>(
    rng: R,
) -> ParseConfig<AlwaysWarnAndUseNewer, R, DefaultTokenRelaxer> {
    ParseConfig {
        prompter: AlwaysWarnAndUseNewer,
        rng,
        token_modifier: DefaultTokenRelaxer,
    }
}

impl<P, R, M> ParseConfig<P, R, M> {
    /// Sets a custom prompter for conflict resolution.
    pub fn prompter<P2: Prompter>(self, prompter: P2) -> ParseConfig<P2, R, M> {
        ParseConfig {
            prompter,
            rng: self.rng,
            token_modifier: self.token_modifier,
        }
    }

    /// Sets a custom random number generator.
    pub fn rng<R2: Rng>(self, rng: R2) -> ParseConfig<P, R2, M> {
        ParseConfig {
            prompter: self.prompter,
            rng,
            token_modifier: self.token_modifier,
        }
    }

    /// Appends an additional token modifier after the current one.
    pub fn append_token_modifier<M2: TokenModifier>(
        self,
        token_modifier: M2,
    ) -> ParseConfig<P, R, SequentialTokenModifier<M, M2>>
    where
        M: TokenModifier,
    {
        ParseConfig {
            prompter: self.prompter,
            rng: self.rng,
            token_modifier: self.token_modifier.then(token_modifier),
        }
    }

    /// Replaces the current token modifier with a new one.
    pub fn override_token_modifier<M2: TokenModifier>(
        self,
        token_modifier: M2,
    ) -> ParseConfig<P, R, M2> {
        ParseConfig {
            prompter: self.prompter,
            rng: self.rng,
            token_modifier,
        }
    }

    /// Transforms the token modifier using the provided function.
    pub fn map_token_modifier<F, M2>(self, f: F) -> ParseConfig<P, R, M2>
    where
        F: FnOnce(M) -> M2,
    {
        ParseConfig {
            prompter: self.prompter,
            rng: self.rng,
            token_modifier: f(self.token_modifier),
        }
    }

    /// Removes the token modifier, using a no-op modifier instead.
    pub fn clean_token_modifier(self) -> ParseConfig<P, R, NoopTokenModifier> {
        self.override_token_modifier(NoopTokenModifier)
    }

    pub(crate) fn build(self) -> (impl TokenProcessor<Output = Bms>, P)
    where
        P: Prompter,
        R: Rng,
    {
        struct AggregateTokenProcessor<R> {
            rng: Rc<RefCell<R>>,
        }
        impl<R: Rng> TokenProcessor for AggregateTokenProcessor<R> {
            type Output = Bms;

            fn process<P: Prompter>(
                &self,
                ctx: &mut parse::token_processor::ProcessContext<'_, '_, P>,
            ) -> Result<Self::Output, ParseErrorWithRange> {
                full_preset::<R>(Rc::clone(&self.rng)).process(ctx)
            }
        }
        (
            AggregateTokenProcessor::<R> {
                rng: Rc::new(RefCell::new(self.rng)),
            },
            self.prompter,
        )
    }
}

/// Parse a BMS file from source text with the specified command preset.
pub fn parse_bms<P: Prompter, R: Rng, M: TokenModifier>(
    source: &str,
    config: ParseConfig<P, R, M>,
) -> BmsOutput {
    let LexOutput {
        mut tokens,
        lex_warnings,
    } = lex::TokenStream::parse_lex(source);

    let mut warnings: Vec<BmsWarning> = lex_warnings.into_iter().map(BmsWarning::Lex).collect();

    config.token_modifier.modify(&mut tokens);
    let parse_output = Bms::from_token_stream(&tokens, config);
    let bms_result = parse_output.bms;
    warnings.extend(
        parse_output
            .parse_warnings
            .into_iter()
            .map(BmsWarning::Parse),
    );

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
        use BmsWarning::{Lex, Parse};
        match self {
            Lex(e) => e.to_report(src),
            Parse(e) => e.to_report(src),
        }
    }
}
