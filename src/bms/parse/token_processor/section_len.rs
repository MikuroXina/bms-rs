//! This module handles the tokens:
//!
//! - `#xxx02:`: Section length ratio channel. `1.0` makes `xxx` section to be 4/4 beat.

use strict_num_extended::FinF64;

use super::{super::prompt::Prompter, ProcessContext, TokenProcessor, filter_message};
use crate::bms::ParseErrorWithRange;
use crate::bms::{
    model::section_len::SectionLenObjects,
    parse::{ParseWarning, Result},
    prelude::*,
};

/// It processes objects on `SectionLen` channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SectionLenProcessor;

impl TokenProcessor for SectionLenProcessor {
    type Output = SectionLenObjects;

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> core::result::Result<Self::Output, ParseErrorWithRange> {
        let mut objects = SectionLenObjects::default();
        ctx.all_tokens(|token, prompter| match token.content() {
            Token::Message {
                track,
                channel,
                message,
            } => Ok(
                Self::on_message(*track, *channel, message.as_ref(), prompter, &mut objects)
                    .err()
                    .map(|warn| warn.into_wrapper(token)),
            ),
            Token::Header { .. } | Token::NotACommand(_) => Ok(None),
        })?;
        Ok(objects)
    }
}

impl SectionLenProcessor {
    fn on_message(
        track: Track,
        channel: Channel,
        message: &str,
        prompter: &impl Prompter,
        objects: &mut SectionLenObjects,
    ) -> Result<()> {
        if channel == Channel::SectionLen {
            let message = filter_message(message);
            let message = message.as_ref();
            let length = message.parse::<FinF64>().map_err(|_| {
                ParseWarning::SyntaxError(format!("Invalid section length: {message}"))
            })?;
            if length < FinF64::ZERO {
                return Err(ParseWarning::SyntaxError(
                    "section length must be non-negative".to_string(),
                ));
            }
            objects.push_section_len_change(SectionLenChangeObj { track, length }, prompter)?;
        }
        Ok(())
    }
}
