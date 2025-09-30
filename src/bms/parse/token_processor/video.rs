#[cfg(feature = "minor-command")]
use std::str::FromStr;
use std::{cell::RefCell, path::Path, rc::Rc};

#[cfg(feature = "minor-command")]
use fraction::GenericFraction;
#[cfg(feature = "minor-command")]
use num::BigUint;

use super::{super::prompt::Prompter, Result, TokenProcessor, ids_from_message};
use crate::bms::{model::Bms, prelude::*};

/// It processes `#VIDEOFILE`, `#MOVIE` and so on definitions and objects on `Seek` channel.
pub struct VideoProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for VideoProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name == "VIDEOFILE" {
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError("expected video filename".into()));
            }
            self.0.borrow_mut().graphics.video_file = Some(Path::new(args).into());
        }
        if name == "MOVIE" {
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError("expected movie filename".into()));
            }
            self.0.borrow_mut().header.movie = Some(Path::new(args).into());
        }
        #[cfg(feature = "minor-command")]
        if name == "VIDEOF/S" {
            let frame_rate = Decimal::from_fraction(
                GenericFraction::<BigUint>::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected f64".into()))?,
            );
            self.0.borrow_mut().graphics.video_fs = Some(frame_rate);
        }
        #[cfg(feature = "minor-command")]
        if name == "VIDEOCOLORS" {
            let colors = args
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected u8".into()))?;
            self.0.borrow_mut().graphics.video_colors = Some(colors);
        }
        #[cfg(feature = "minor-command")]
        if name == "VIDEODLY" {
            let delay = Decimal::from_fraction(
                GenericFraction::<BigUint>::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected f64".into()))?,
            );
            self.0.borrow_mut().graphics.video_dly = Some(delay);
        }
        #[cfg(feature = "minor-command")]
        if name.starts_with("SEEK") {
            use fraction::GenericFraction;
            use num::BigUint;

            let id = name.trim_start_matches("SEEK");
            let ms = Decimal::from_fraction(
                GenericFraction::<BigUint>::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected decimal".into()))?,
            );
            let id = ObjId::try_from(id)?;

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
