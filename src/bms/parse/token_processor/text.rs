//! This module handles the tokens:
//!
//! - `#TEXT[01-ZZ] text` - Text definition shown on playing. It can be double-quoted.
//! - `#SONG[01-ZZ] text` - Text definition. Obsolete.
//! - `#xxx99:` - Text channel.

use std::{cell::RefCell, rc::Rc};

use super::{
    super::prompt::Prompter, TokenProcessor, TokenProcessorOutput, all_tokens_with_range,
    parse_obj_ids,
};
use crate::{
    bms::{model::text::TextObjects, prelude::*},
    util::StrExtension,
};

/// It processes `#TEXTxx` definition and objects on `Text` channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextProcessor {
    case_sensitive_obj_id: Rc<RefCell<bool>>,
}

impl TextProcessor {
    pub fn new(case_sensitive_obj_id: &Rc<RefCell<bool>>) -> Self {
        Self {
            case_sensitive_obj_id: Rc::clone(case_sensitive_obj_id),
        }
    }
}

impl TokenProcessor for TextProcessor {
    type Output = TextObjects;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> TokenProcessorOutput<Self::Output> {
        let mut objects = TextObjects::default();
        let mut extra_warnings: Vec<ParseWarningWithRange> = Vec::new();
        let TokenProcessorOutput {
            output: res,
            mut warnings,
        } = all_tokens_with_range(input, |token| match token.content() {
            Token::Header { name, args } => Ok(self
                .on_header(name.as_ref(), args.as_ref(), prompter, &mut objects)
                .err()),
            Token::Message {
                track,
                channel,
                message,
            } => self
                .on_message(
                    *track,
                    *channel,
                    message.as_ref().into_wrapper(token),
                    prompter,
                    &mut objects,
                )
                .map_or_else(
                    |warn| Ok(Some(warn)),
                    |w| {
                        extra_warnings.extend(w);
                        Ok(None)
                    },
                ),
            Token::NotACommand(_) => Ok(None),
        });
        warnings.extend(extra_warnings);
        TokenProcessorOutput {
            output: res.map(|_| objects),
            warnings,
        }
    }
}

impl TextProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut TextObjects,
    ) -> core::result::Result<(), ParseWarning> {
        if let Some(id) = name
            .strip_prefix_ignore_case("TEXT")
            .or_else(|| name.strip_prefix_ignore_case("SONG"))
        {
            let id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;

            if let Some(older) = objects.texts.get_mut(&id) {
                prompter
                    .handle_def_duplication(DefDuplication::Text {
                        id,
                        older,
                        newer: args,
                    })
                    .apply_def(older, args.to_string(), id)?;
            } else {
                objects.texts.insert(id, args.to_string());
            }
        }
        Ok(())
    }

    fn on_message(
        &self,
        track: Track,
        channel: Channel,
        message: SourceRangeMixin<&str>,
        prompter: &impl Prompter,
        objects: &mut TextObjects,
    ) -> core::result::Result<Vec<ParseWarningWithRange>, ParseWarning> {
        let mut warnings: Vec<ParseWarningWithRange> = Vec::new();
        if channel == Channel::Text {
            let (pairs, w) = parse_obj_ids(track, message, &self.case_sensitive_obj_id);
            warnings.extend(w);
            for (time, text_id) in pairs {
                let text = objects
                    .texts
                    .get(&text_id)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(text_id))?;
                objects.push_text_event(TextObj { time, text }, prompter)?;
            }
        }
        Ok(warnings)
    }
}
