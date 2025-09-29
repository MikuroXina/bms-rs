use std::{cell::RefCell, rc::Rc};

use super::{
    super::prompt::{DefDuplication, Prompter},
    ParseWarning, Result, TokenProcessor, ids_from_message,
};
use crate::bms::{model::Bms, prelude::*};

/// It processes `#OPTION` and `#CHANGEOPTIONxx` definitions and objects on `Option` channel.
pub struct OptionProcessor<'a, P, T>(Rc<RefCell<Bms<T>>>, &'a P);

impl<P: Prompter, T> TokenProcessor for OptionProcessor<'_, P, T> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        #[cfg(feature = "minor-command")]
        if name == "OPTION" {
            self.0
                .borrow_mut()
                .others
                .options
                .get_or_insert_with(Vec::new)
                .push(args.to_string());
        }
        #[cfg(feature = "minor-command")]
        if name.starts_with("CHANGEOPTION") {
            let id = name.trim_start_matches("CHANGEOPTION");
            let id = ObjId::try_from(id).map_err(|_| {
                ParseWarning::SyntaxError(format!("expected object id but found {id:?}"))
            })?;
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
        match channel {
            #[cfg(feature = "minor-command")]
            Channel::Option => {
                for (time, option_id) in ids_from_message(track, message, |w| self.1.warn(w)) {
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
            #[cfg(feature = "minor-command")]
            Channel::ChangeOption => {
                for (_time, obj) in ids_from_message(track, message, |w| self.1.warn(w)) {
                    let _option = self
                        .0
                        .borrow()
                        .others
                        .change_options
                        .get(&obj)
                        .cloned()
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    // Here we can add logic to handle ChangeOption
                    // Currently just ignored because change_options are already stored in notes
                }
            }
            _ => {}
        }
        Ok(())
    }
}
