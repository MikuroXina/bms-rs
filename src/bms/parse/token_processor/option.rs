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
    bms::{error::Result, model::option::OptionObjects, prelude::*},
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
        Vec<ControlFlowErrorWithRange>,
    ) {
        let mut objects = OptionObjects::default();
        let mut all_warnings = Vec::new();
        let mut all_control_flow_errors = Vec::new();
        let (_, warnings, control_flow_errors) = all_tokens_with_range(input, |token| {
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
        all_control_flow_errors.extend(control_flow_errors);
        (objects, all_warnings, all_control_flow_errors)
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
        match channel {
            Channel::OptionChange => {
                let (obj_ids, parse_warnings) = parse_obj_ids_with_warnings(
                    track,
                    message.clone(),
                    &self.case_sensitive_obj_id,
                );
                let option_warnings = obj_ids.into_iter().flat_map(|(time, option_id)| {
                    objects
                        .change_options
                        .get(&option_id)
                        .cloned()
                        .map_or_else(
                            || {
                                Some(
                                    ParseWarning::UndefinedObject(option_id).into_wrapper(&message),
                                )
                            },
                            |option| {
                                objects
                                    .push_option_event(OptionObj { time, option }, prompter)
                                    .err()
                                    .map(|warning| warning.into_wrapper(&message))
                            },
                        )
                        .into_iter()
                });
                parse_warnings.into_iter().chain(option_warnings).collect()
            }
            _ => Vec::new(),
        }
    }
}
