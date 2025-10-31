//! This module handles the tokens:
//!
//! - `#xxx02:`: Section length ratio channel. `1.0` makes `xxx` section to be 4/4 beat.

use std::str::FromStr;

use fraction::GenericFraction;

use super::{super::prompt::Prompter, TokenProcessor, all_tokens, filter_message};
use crate::bms::{
    error::{ParseErrorWithRange, ParseWarning},
    model::section_len::SectionLenObjects,
    prelude::*,
};

/// It processes objects on `SectionLen` channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SectionLenProcessor;

impl TokenProcessor for SectionLenProcessor {
    type Output = SectionLenObjects;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> (
        Self::Output,
        Vec<ParseWarningWithRange>,
        Vec<ParseErrorWithRange>,
    ) {
        let mut objects = SectionLenObjects::default();
        let mut all_warnings = Vec::new();
        let (_, warnings, errors) = all_tokens(input, prompter, |token| {
            Ok(match token {
                Token::Message {
                    track,
                    channel,
                    message,
                } => {
                    let message_warnings =
                        self.on_message(*track, *channel, message.as_ref(), prompter, &mut objects);
                    all_warnings.extend(message_warnings);
                    None
                }
                Token::Header { .. } | Token::NotACommand(_) => None,
            })
        });
        all_warnings.extend(warnings);
        (objects, all_warnings, errors)
    }
}

impl SectionLenProcessor {
    fn on_message(
        &self,
        track: Track,
        channel: Channel,
        message: &str,
        prompter: &impl Prompter,
        objects: &mut SectionLenObjects,
    ) -> Vec<ParseWarningWithRange> {
        let mut warnings = Vec::new();
        if channel == Channel::SectionLen {
            let message = filter_message(message);
            let message = message.as_ref();
            let fraction_result = GenericFraction::from_str(message).map_err(|_| {
                ParseWarning::SyntaxError(format!("Invalid section length: {message}"))
            });
            let length = match fraction_result {
                Ok(fraction) => Decimal::from(Decimal::from_fraction(fraction)),
                Err(warning) => {
                    warnings.push(warning.into_wrapper(&SourceRangeMixin::new(message, 0..0)));
                    return warnings;
                }
            };
            if length <= Decimal::from(0u64) {
                warnings.push(
                    ParseWarning::SyntaxError(
                        "section length must be greater than zero".to_string(),
                    )
                    .into_wrapper(&SourceRangeMixin::new(message, 0..0)),
                );
                return warnings;
            }
            if let Err(warning) =
                objects.push_section_len_change(SectionLenChangeObj { track, length }, prompter)
            {
                warnings.push(warning.into_wrapper(&SourceRangeMixin::new(message, 0..0)));
            }
        }
        warnings
    }
}
