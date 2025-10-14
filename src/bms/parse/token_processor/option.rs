//! This module handles the tokens:
//!
//! - `#OPTION option_id` - Option for a BMS player software. In most cases, it has also the vendor-prefix.
//! - `#CHANGEOPTION[01-ZZ] option_id` - Option change definition for a BMS player software. In most cases, it has also the vendor-prefix.
//! - `#xxxA6:` - Option change channel.
#![cfg(feature = "minor-command")]

use super::{
    super::prompt::{DefDuplication, Prompter},
    ParseWarning, Result, TokenProcessor, ids_from_message,
};
use crate::bms::{model::Bms, prelude::*};
use std::ops::ControlFlow;
use std::{cell::RefCell, rc::Rc};

/// It processes `#OPTION` and `#CHANGEOPTIONxx` definitions and objects on `Option` channel.
pub struct OptionProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for OptionProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> ControlFlow<Result<()>> {
        match name.to_ascii_uppercase().as_str() {
            "OPTION" => {
                self.0
                    .borrow_mut()
                    .others
                    .options
                    .get_or_insert_with(Vec::new)
                    .push(args.to_string());
                return ControlFlow::Break(Ok(()));
            }
            change_option if change_option.starts_with("CHANGEOPTION") => {
                let id = &name["CHANGEOPTION".len()..];
                let id = match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                    Ok(v) => v,
                    Err(e) => return ControlFlow::Break(Err(e)),
                };
                if let Some(older) = self.0.borrow_mut().others.change_options.get_mut(&id) {
                    if let Err(e) = self
                        .1
                        .handle_def_duplication(DefDuplication::ChangeOption {
                            id,
                            older,
                            newer: args,
                        })
                        .apply_def(older, args.to_string(), id)
                    {
                        return ControlFlow::Break(Err(e));
                    }
                } else {
                    self.0
                        .borrow_mut()
                        .others
                        .change_options
                        .insert(id, args.to_string());
                }
                return ControlFlow::Break(Ok(()));
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> ControlFlow<Result<()>> {
        if channel == Channel::OptionChange {
            for (time, option_id) in ids_from_message(
                track,
                message,
                self.0.borrow().header.case_sensitive_obj_id,
                |w| self.1.warn(w),
            ) {
                let option = match self
                    .0
                    .borrow()
                    .others
                    .change_options
                    .get(&option_id)
                    .cloned()
                {
                    Some(v) => v,
                    None => {
                        return ControlFlow::Break(Err(ParseWarning::UndefinedObject(option_id)));
                    }
                };
                if let Err(e) = self
                    .0
                    .borrow_mut()
                    .notes
                    .push_option_event(OptionObj { time, option }, self.1)
                {
                    return ControlFlow::Break(Err(e));
                }
            }
            return ControlFlow::Break(Ok(()));
        }
        ControlFlow::Continue(())
    }
}
