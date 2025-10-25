//! This module handles the tokens:
//!
//! - `#TEXT[01-ZZ] text` - Text definition shown on playing. It can be double-quoted.
//! - `#SONG[01-ZZ] text` - Text definition. Obsolete.
//! - `#xxx99:` - Text channel.

use std::{cell::RefCell, rc::Rc};

use super::{
    super::prompt::Prompter, TokenProcessor, TokenProcessorResult, all_tokens_with_range,
    parse_obj_ids,
};
use crate::bms::{error::Result, model::text::TextObjects, prelude::*};

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
    ) -> TokenProcessorResult<Self::Output> {
        let mut objects = TextObjects::default();
        all_tokens_with_range(input, prompter, |token| {
            Ok(match token.content() {
                Token::Header { name, args } => self
                    .on_header(name.as_ref(), args.as_ref(), prompter, &mut objects)
                    .err(),
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
                    .err(),
                Token::NotACommand(_) => None,
            })
        })?;
        Ok(objects)
    }
}

impl TextProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut TextObjects,
    ) -> Result<()> {
        let upper = name.to_ascii_uppercase();
        if upper.starts_with("TEXT") || upper.starts_with("SONG") {
            let id = &name["TEXT".len()..];
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
    ) -> Result<()> {
        if channel == Channel::Text {
            for (time, text_id) in
                parse_obj_ids(track, message, prompter, &self.case_sensitive_obj_id)
            {
                let text = objects
                    .texts
                    .get(&text_id)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(text_id))?;
                objects.push_text_event(TextObj { time, text }, prompter)?;
            }
        }
        Ok(())
    }
}
