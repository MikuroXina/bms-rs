//! This module handles the tokens:
//!
//! - `#TEXT[01-ZZ] text` - Text definition shown on playing. It can be double-quoted.
//! - `#SONG[01-ZZ] text` - Text definition. Obsolete.
//! - `#xxx99:` - Text channel.

use std::{cell::RefCell, rc::Rc};

use super::{super::prompt::Prompter, ProcessContext, TokenProcessor, parse_obj_ids};
use crate::bms::parse::ControlFlowErrorWithRange;
use crate::{
    bms::{
        model::text::TextObjects,
        parse::{ParseWarning, Result},
        prelude::*,
    },
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

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> core::result::Result<Self::Output, ControlFlowErrorWithRange> {
        let mut objects = TextObjects::default();
        ctx.all_tokens(|token, prompter| match token.content() {
            Token::Header { name, args } => {
                match self.on_header(name.as_ref(), args.as_ref(), prompter, &mut objects) {
                    Ok(()) => Ok(Vec::new()),
                    Err(warn) => Ok(vec![warn.into_wrapper(token)]),
                }
            }
            Token::Message {
                track,
                channel,
                message,
            } => {
                match self.on_message(
                    *track,
                    *channel,
                    message.as_ref().into_wrapper(token),
                    prompter,
                    &mut objects,
                ) {
                    Ok(ws) => Ok(ws),
                    Err(warn) => Ok(vec![warn.into_wrapper(token)]),
                }
            }
            Token::NotACommand(_) => Ok(Vec::new()),
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
    ) -> Result<Vec<ParseWarningWithRange>> {
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
