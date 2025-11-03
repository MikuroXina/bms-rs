//! This module handles the tokens:
//!
//! - `#OPTION option_id` - Option for a BMS player software. In most cases, it has also the vendor-prefix.
//! - `#CHANGEOPTION[01-ZZ] option_id` - Option change definition for a BMS player software. In most cases, it has also the vendor-prefix.
//! - `#xxxA6:` - Option change channel.

use std::{cell::RefCell, rc::Rc};

use super::{
    super::prompt::{DefDuplication, Prompter},
    TokenProcessor, TokenProcessorOutput, all_tokens_with_range, parse_obj_ids,
};
use crate::{
    bms::{
        error::{ParseWarning, Result},
        model::option::OptionObjects,
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

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> TokenProcessorOutput<Self::Output> {
        let mut objects = OptionObjects::default();
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
            } => match self.on_message(
                *track,
                *channel,
                message.as_ref().into_wrapper(token),
                prompter,
                &mut objects,
            ) {
                Ok(w) => {
                    extra_warnings.extend(w);
                    Ok(None)
                }
                Err(warn) => Ok(Some(warn)),
            },
            Token::NotACommand(_) => Ok(None),
        });
        warnings.extend(extra_warnings);
        match res {
            Ok(()) => TokenProcessorOutput {
                output: Ok(objects),
                warnings,
            },
            Err(e) => TokenProcessorOutput {
                output: Err(e),
                warnings,
            },
        }
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
            let (pairs, mut w) = parse_obj_ids(track, message, &self.case_sensitive_obj_id);
            warnings.append(&mut w);
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
