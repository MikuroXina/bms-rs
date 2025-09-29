use std::{cell::RefCell, rc::Rc};

use super::{super::prompt::Prompter, Result, TokenProcessor, ids_from_message};
use crate::bms::{model::Bms, prelude::*};

/// It processes `#TEXTxx` definition and objects on `Text` channel.
pub struct TextProcessor<'a, P, T>(Rc<RefCell<Bms<T>>>, &'a P);

impl<P: Prompter, T: KeyLayoutMapper> TokenProcessor for TextProcessor<'_, P, T> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name.starts_with("TEXT") || name.starts_with("SONG") {
            let id = if name.starts_with("TEXT") {
                name.trim_start_matches("TEXT")
            } else {
                name.trim_start_matches("#SONG")
            };
            let id = ObjId::try_from(id).map_err(|id| {
                ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
            })?;

            if let Some(older) = self.0.borrow_mut().others.texts.get_mut(&id) {
                self.1
                    .handle_def_duplication(DefDuplication::Text {
                        id,
                        older,
                        newer: args,
                    })
                    .apply_def(older, args.to_string(), id)?;
            } else {
                self.0
                    .borrow_mut()
                    .others
                    .texts
                    .insert(id, args.to_string());
            }
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        match channel {
            Channel::Text => {
                for (time, text_id) in ids_from_message(track, message, |w| self.1.warn(w)) {
                    let text = self
                        .0
                        .borrow()
                        .others
                        .texts
                        .get(&text_id)
                        .cloned()
                        .ok_or(ParseWarning::UndefinedObject(text_id))?;
                    self.0
                        .borrow_mut()
                        .notes
                        .push_text_event(TextObj { time, text }, self.1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
