//! This module handles the tokens:
//!
//! - `#TEXT[01-ZZ] text` - Text definition shown on playing. It can be double-quoted.
//! - `#SONG[01-ZZ] text` - Text definition. Obsolete.
//! - `#xxx99:` - Text channel.
use std::{cell::RefCell, rc::Rc};

use super::{super::prompt::Prompter, Result, TokenProcessor, ids_from_message};
use crate::bms::{model::Bms, prelude::*};
use std::ops::ControlFlow;

/// It processes `#TEXTxx` definition and objects on `Text` channel.
pub struct TextProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for TextProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> ControlFlow<Result<()>> {
        let upper = name.to_ascii_uppercase();
        if upper.starts_with("TEXT") || upper.starts_with("SONG") {
            let id = &name["TEXT".len()..];
            let id = match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                Ok(v) => v,
                Err(e) => return ControlFlow::Break(Err(e)),
            };

            if let Some(older) = self.0.borrow_mut().others.texts.get_mut(&id) {
                if let Err(e) = self
                    .1
                    .handle_def_duplication(DefDuplication::Text {
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
                    .texts
                    .insert(id, args.to_string());
            }
            return ControlFlow::Break(Ok(()));
        }
        ControlFlow::Continue(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> ControlFlow<Result<()>> {
        if channel == Channel::Text {
            let is_sensitive = self.0.borrow().header.case_sensitive_obj_id;
            for (time, text_id) in
                ids_from_message(track, message, is_sensitive, |w| self.1.warn(w))
            {
                let text = match self.0.borrow().others.texts.get(&text_id).cloned() {
                    Some(v) => v,
                    None => return ControlFlow::Break(Err(ParseWarning::UndefinedObject(text_id))),
                };
                if let Err(e) = self
                    .0
                    .borrow_mut()
                    .notes
                    .push_text_event(TextObj { time, text }, self.1)
                {
                    return ControlFlow::Break(Err(e));
                }
            }
            return ControlFlow::Break(Ok(()));
        }
        ControlFlow::Continue(())
    }
}
