//! This module handles the tokens:
//!
//! - `#OPTION option_id` - Option for a BMS player software. In most cases, it has also the vendor-prefix.
//! - `#CHANGEOPTION[01-ZZ] option_id` - Option change definition for a BMS player software. In most cases, it has also the vendor-prefix.
//! - `#xxxA6:` - Option change channel.
#![cfg(feature = "minor-command")]

use super::{
    super::{
        Result,
        prompt::{DefDuplication, Prompter},
    },
    ParseWarning, TokenProcessor, TokenProcessorResult, all_tokens, ids_from_message,
};
use crate::bms::{model::Bms, prelude::*};
use std::{cell::RefCell, rc::Rc};

/// It processes `#OPTION` and `#CHANGEOPTIONxx` definitions and objects on `Option` channel.
pub struct OptionProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for OptionProcessor<'_, P> {
    fn process(&self, input: &mut &[TokenWithRange<'_>]) -> TokenProcessorResult {
        all_tokens(input, |token| {
            Ok(match token {
                Token::Header { name, args } => self.on_header(name.as_ref(), args.as_ref()).err(),
                Token::Message {
                    track,
                    channel,
                    message,
                } => self.on_message(*track, *channel, message.as_ref()).err(),
                Token::NotACommand(_) => None,
            })
        })
    }
}

impl<P: Prompter> OptionProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "OPTION" => {
                self.0
                    .borrow_mut()
                    .others
                    .options
                    .get_or_insert_with(Vec::new)
                    .push(args.to_string());
            }
            change_option if change_option.starts_with("CHANGEOPTION") => {
                let id = &name["CHANGEOPTION".len()..];
                let id = ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id)?;
                if let Some(older) = self.0.borrow_mut().others.change_options.get_mut(&id) {
                    self.1
                        .handle_def_duplication(DefDuplication::ChangeOption {
                            id,
                            older,
                            newer: args,
                        })
                        .apply_def(older, args.to_string(), id)?;
                } else {
                    self.0
                        .borrow_mut()
                        .others
                        .change_options
                        .insert(id, args.to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if channel == Channel::OptionChange {
            for (time, option_id) in ids_from_message(
                track,
                message,
                self.0.borrow().header.case_sensitive_obj_id,
                |w| self.1.warn(w),
            ) {
                let option = self
                    .0
                    .borrow()
                    .others
                    .change_options
                    .get(&option_id)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(option_id))?;
                self.0
                    .borrow_mut()
                    .notes
                    .push_option_event(OptionObj { time, option }, self.1)?;
            }
        }
        Ok(())
    }
}
