//! This module handles the tokens:
//!
//! - `#xxx02:`: Section length ratio channel. `1.0` makes `xxx` section to be 4/4 beat.

use std::str::FromStr;

use fraction::GenericFraction;

use super::{
    super::prompt::Prompter, TokenProcessor, TokenProcessorResult, all_tokens, filter_message,
};
use crate::bms::{
    error::{ParseWarning, Result},
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
    ) -> TokenProcessorResult<Self::Output> {
        let mut objects = SectionLenObjects::default();
        all_tokens(input, prompter, |token| {
            Ok(match token {
                Token::Message {
                    track,
                    channel,
                    message,
                } => self
                    .on_message(*track, *channel, message.as_ref(), prompter, &mut objects)
                    .err(),
                Token::Header { .. } | Token::NotACommand(_) => None,
            })
        })?;
        Ok(objects)
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
    ) -> Result<()> {
        if channel == Channel::SectionLen {
            let message = filter_message(message);
            let message = message.as_ref();
            let length = Decimal::from(Decimal::from_fraction(
                GenericFraction::from_str(message).map_err(|_| {
                    ParseWarning::SyntaxError(format!("Invalid section length: {message}"))
                })?,
            ));
            if length <= Decimal::from(0u64) {
                return Err(ParseWarning::SyntaxError(
                    "section length must be greater than zero".to_string(),
                ));
            }
            objects.push_section_len_change(SectionLenChangeObj { track, length }, prompter)?;
        }
        Ok(())
    }
}
