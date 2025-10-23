//! This module handles the tokens:
//!
//! - `#OPTION option_id` - Option for a BMS player software. In most cases, it has also the vendor-prefix.
//! - `#CHANGEOPTION[01-ZZ] option_id` - Option change definition for a BMS player software. In most cases, it has also the vendor-prefix.
//! - `#xxxA6:` - Option change channel.
#![cfg(feature = "minor-command")]

use super::{
    super::prompt::{DefDuplication, Prompter},
    TokenProcessor, TokenProcessorResult, all_tokens_with_range, parse_obj_ids,
};
use crate::bms::{
    error::{ParseWarning, Result},
    model::option::OptionObjects,
    prelude::*,
};
use std::{cell::RefCell, rc::Rc};

/// It processes `#OPTION` and `#CHANGEOPTIONxx` definitions and objects on `Option` channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionProcessor {
    case_sensitive_obj_id: Rc<RefCell<bool>>,
}

impl OptionProcessor {
    pub fn new(case_sensitive_obj_id: &Rc<RefCell<bool>>) -> Self {
        Self {
            case_sensitive_obj_id: Rc::clone(case_sensitive_obj_id),
        }
    }
}

impl TokenProcessor for OptionProcessor {
    type Output = OptionObjects;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> TokenProcessorResult<Self::Output> {
        let mut objects = OptionObjects::default();
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

impl OptionProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut OptionObjects,
    ) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "OPTION" => {
                objects
                    .options
                    .get_or_insert_with(Vec::new)
                    .push(args.to_string());
            }
            change_option if change_option.starts_with("CHANGEOPTION") => {
                let id = &name["CHANGEOPTION".len()..];
                let id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;
                if let Some(older) = objects.change_options.get_mut(&id) {
                    prompter
                        .handle_def_duplication(DefDuplication::ChangeOption {
                            id,
                            older,
                            newer: args,
                        })
                        .apply_def(older, args.to_string(), id)?;
                } else {
                    objects.change_options.insert(id, args.to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn on_message(
        &self,
        track: Track,
        channel: Channel,
        message: SourceRangeMixin<&str>,
        prompter: &impl Prompter,
        objects: &mut OptionObjects,
    ) -> Result<()> {
        if channel == Channel::OptionChange {
            for (time, option_id) in
                parse_obj_ids(track, message, prompter, &self.case_sensitive_obj_id)
            {
                let option = objects
                    .change_options
                    .get(&option_id)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(option_id))?;
                objects.push_option_event(OptionObj { time, option }, prompter)?;
            }
        }
        Ok(())
    }
}
