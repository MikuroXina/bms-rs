//! This module handles the tokens:
//!
//! - `#OPTION option_id` - Option for a BMS player software. In most cases, it has also the vendor-prefix.
//! - `#CHANGEOPTION[01-ZZ] option_id` - Option change definition for a BMS player software. In most cases, it has also the vendor-prefix.
//! - `#xxxA6:` - Option change channel.

use std::{cell::RefCell, rc::Rc};

use super::{
    super::prompt::{DefDuplication, Prompter},
    TokenProcessor, all_tokens_with_range, parse_obj_ids_with_warnings,
};
use crate::{
    bms::{
        error::{ControlFlowWarningWithRange, Result},
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
    ) -> (
        Self::Output,
        Vec<ParseWarningWithRange>,
        Vec<ControlFlowWarningWithRange>,
    ) {
        let mut objects = OptionObjects::default();
        let mut all_warnings = Vec::new();
        let (_, warnings, errors) = all_tokens_with_range(input, prompter, |token| {
            Ok(match token.content() {
                Token::Header { name, args } => self
                    .on_header(name.as_ref(), args.as_ref(), prompter, &mut objects)
                    .err(),
                Token::Message {
                    track,
                    channel,
                    message,
                } => {
                    let message_warnings = self.on_message(
                        *track,
                        *channel,
                        message.as_ref().into_wrapper(token),
                        prompter,
                        &mut objects,
                    );
                    all_warnings.extend(message_warnings);
                    None
                }
                Token::NotACommand(_) => None,
            })
        });
        all_warnings.extend(warnings);
        (objects, all_warnings, errors)
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
    ) -> Vec<ParseWarningWithRange> {
        let mut warnings = Vec::new();
        if channel == Channel::OptionChange {
            let (obj_ids, parse_warnings) = parse_obj_ids_with_warnings(
                track,
                message.clone(),
                prompter,
                &self.case_sensitive_obj_id,
            );
            warnings.extend(parse_warnings);
            for (time, option_id) in obj_ids {
                let option = match objects.change_options.get(&option_id).cloned() {
                    Some(option) => option,
                    None => {
                        warnings
                            .push(ParseWarning::UndefinedObject(option_id).into_wrapper(&message));
                        continue;
                    }
                };
                if let Err(warning) =
                    objects.push_option_event(OptionObj { time, option }, prompter)
                {
                    warnings.push(warning.into_wrapper(&message));
                }
            }
        }
        warnings
    }
}
