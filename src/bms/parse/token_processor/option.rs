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
use std::{cell::RefCell, rc::Rc};

/// It processes `#OPTION` and `#CHANGEOPTIONxx` definitions and objects on `Option` channel.
pub struct OptionProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for OptionProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name == "OPTION" {
            self.0
                .borrow_mut()
                .others
                .options
                .get_or_insert_with(Vec::new)
                .push(args.to_string());
        }
        if name.starts_with("CHANGEOPTION") {
            let id = name.trim_start_matches("CHANGEOPTION");
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
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if let Channel::OptionChange = channel {
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
