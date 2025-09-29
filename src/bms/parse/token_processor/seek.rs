use std::{cell::RefCell, rc::Rc, str::FromStr};

use super::{super::prompt::Prompter, Result, TokenProcessor, ids_from_message};
use crate::bms::{model::Bms, prelude::*};

/// It processes `#SEEKxx` definition and objects on `Seek` channel.
pub struct SeekProcessor<'a, P, T>(Rc<RefCell<Bms<T>>>, &'a P);

impl<P: Prompter, T: KeyLayoutMapper> TokenProcessor for SeekProcessor<'_, P, T> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        #[cfg(feature = "minor-command")]
        if name.starts_with("#SEEK") {
            use fraction::GenericFraction;
            use num::BigUint;

            let id = name.trim_start_matches("#SEEK");
            let ms = Decimal::from_fraction(
                GenericFraction::<BigUint>::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected decimal".into()))?,
            );
            let id = ObjId::try_from(id).map_err(|id| {
                ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
            })?;

            if let Some(older) = self.0.borrow_mut().others.seek_events.get_mut(&id) {
                self.1
                    .handle_def_duplication(DefDuplication::SeekEvent {
                        id,
                        older,
                        newer: &ms,
                    })
                    .apply_def(older, ms, id)?;
            } else {
                self.0.borrow_mut().others.seek_events.insert(id, ms);
            }
        }
        todo!()
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        match channel {
            #[cfg(feature = "minor-command")]
            Channel::Seek => {
                for (time, seek_id) in ids_from_message(track, message, |w| self.1.warn(w)) {
                    let position = self
                        .0
                        .borrow()
                        .others
                        .seek_events
                        .get(&seek_id)
                        .cloned()
                        .ok_or(ParseWarning::UndefinedObject(seek_id))?;
                    self.0
                        .borrow_mut()
                        .notes
                        .push_seek_event(SeekObj { time, position }, self.1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
