//! Parsing Bms from [`TokenStream`].
//!
//! Raw [String] == [lex] ==> [`TokenStream`] (in [`BmsLexOutput`]) == [parse] ==> [Bms] (in
//! [`BmsParseOutput`])

pub mod check_playing;
pub mod prompt;
pub mod token_processor;
pub mod validity;

#[cfg(feature = "diagnostics")]
use crate::diagnostics::{SimpleSource, ToAriadne};
#[cfg(feature = "diagnostics")]
use ariadne::{Color, Label, Report, ReportKind};

use crate::bms::{
    command::channel::mapper::KeyLayoutMapper, lex::token::TokenWithRange, model::Bms,
};

use self::{prompt::Prompter, token_processor::TokenProcessor};

use super::{
    ParseConfig,
    error::{ParseError, ParseErrorWithRange, ParseWarningWithRange},
    rng::Rng,
};

/// Bms Parse Output
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[must_use]
pub struct ParseOutput {
    /// The output Bms.
    pub bms: Result<Bms, ParseError>,
    /// Warnings that occurred during parsing.
    pub parse_warnings: Vec<ParseWarningWithRange>,
}

impl Bms {
    /// Parses a token stream into [`Bms`] without AST.
    pub fn from_token_stream<'a, T: KeyLayoutMapper, P: Prompter, R: Rng>(
        token_iter: impl IntoIterator<Item = &'a TokenWithRange<'a>>,
        config: ParseConfig<T, P, R>,
    ) -> ParseOutput {
        let tokens: Vec<_> = token_iter.into_iter().collect();
        let mut tokens_slice = tokens.as_slice();
        let (proc, prompter) = config.build();
        let res = proc
            .process(&mut tokens_slice, &prompter)
            .map_err(|e: ParseErrorWithRange| e.content().clone());
        match res {
            Ok((bms, parse_warnings)) => ParseOutput {
                bms: Ok(bms),
                parse_warnings,
            },
            Err(err) => ParseOutput {
                bms: Err(err),
                parse_warnings: Vec::new(),
            },
        }
    }
}

#[cfg(feature = "diagnostics")]
impl ToAriadne for ParseWarningWithRange {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        let (start, end) = self.as_span();
        let filename = src.name().to_string();
        Report::build(ReportKind::Warning, (filename.clone(), start..end))
            .with_message("parse: ".to_string() + &self.content().to_string())
            .with_label(Label::new((filename, start..end)).with_color(Color::Blue))
            .finish()
    }
}
