//! This module handles the tokens:
//!
//! - `#OPTION option_id` - Option for a BMS player software. In most cases, it has also the vendor-prefix.
//! - `#CHANGEOPTION[01-ZZ] option_id` - Option change definition for a BMS player software. In most cases, it has also the vendor-prefix.
//! - `#xxxA6:` - Option change channel.

use std::{cell::RefCell, rc::Rc};

use super::{
    super::prompt::{DefDuplication, Prompter},
    ProcessContext, TokenProcessor, parse_obj_ids,
};
use crate::bms::ControlFlowErrorWithRange;
use crate::{
    bms::{
        model::option::OptionObjects,
        parse::{ParseWarning, Result},
        prelude::*,
    },
    util::StrExtension,
};

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

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> core::result::Result<Self::Output, ControlFlowErrorWithRange> {
        let mut objects = OptionObjects::default();
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

impl OptionProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut OptionObjects,
    ) -> Result<()> {
        if name.eq_ignore_ascii_case("OPTION") {
            objects
                .options
                .get_or_insert_with(Vec::new)
                .push(args.to_string());
        }
        if let Some(id) = name.strip_prefix_ignore_case("CHANGEOPTION") {
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
        Ok(())
    }

    fn on_message(
        &self,
        track: Track,
        channel: Channel,
        message: SourceRangeMixin<&str>,
        prompter: &impl Prompter,
        objects: &mut OptionObjects,
    ) -> Result<Vec<ParseWarningWithRange>> {
        let mut warnings: Vec<ParseWarningWithRange> = Vec::new();
        if channel == Channel::OptionChange {
            let (pairs, w) = parse_obj_ids(track, message, &self.case_sensitive_obj_id);
            warnings.extend(w);
            for (time, option_id) in pairs {
                let option = objects
                    .change_options
                    .get(&option_id)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(option_id))?;
                objects.push_option_event(OptionObj { time, option }, prompter)?;
            }
        }
        Ok(warnings)
    }
}
