//! This module handles the tokens:
//!
//! - `#xxx02:`: Section length ratio channel. `1.0` makes `xxx` section to be 4/4 beat.

use std::str::FromStr;

use fraction::GenericFraction;

use super::{super::prompt::Prompter, ProcessContext, TokenProcessor, filter_message};
use crate::bms::ParseErrorWithRange;
use crate::bms::{model::section_len::SectionLenObjects, parse::ParseWarning, prelude::*};

/// It processes objects on `SectionLen` channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SectionLenProcessor;

impl TokenProcessor for SectionLenProcessor {
    type Output = SectionLenObjects;

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> Result<Self::Output, ParseErrorWithRange> {
        let mut objects = SectionLenObjects::default();
        ctx.all_tokens(|token, prompter| match token.content() {
            Token::Message {
                track,
                channel,
                message,
            } => {
                match self.on_message(*track, *channel, message.as_ref(), prompter, &mut objects) {
                    Ok(()) => Ok(Vec::new()),
                    Err(warn) => Ok(vec![warn.into_wrapper(token)]),
                }
            }
            Token::Header { .. } | Token::NotACommand(_) => Ok(Vec::new()),
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
    ) -> core::result::Result<(), ParseWarning> {
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
