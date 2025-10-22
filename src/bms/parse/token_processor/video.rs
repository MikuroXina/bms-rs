//! This module handles the tokens:
//!
//! - `#VIDEOFILE filename` / `#MOVIE filename` - The video file path played as BGA.
//! - `#VIDEOf/s n` - Specifies playing frame rate of the video BGA.
//! - `#VIDEOCOLORS n` - Definies color palette (sample size) of the video BGA.
//! - `#VIDEODLY n` - Defines the start frame of playing the video BGA.
//! - `#SEEK[00-ZZ] n` - It controls playing time of the video BGA. Obsolete.
//! - `#xxx:05` - Video seek channel. Obsolete.
#[cfg(feature = "minor-command")]
use std::str::FromStr;
use std::{cell::RefCell, path::Path, rc::Rc};

#[cfg(feature = "minor-command")]
use fraction::GenericFraction;
#[cfg(feature = "minor-command")]
use num::BigUint;

#[cfg(feature = "minor-command")]
use super::ids_from_message;
use super::{
    super::{Result, prompt::Prompter},
    TokenProcessor, TokenProcessorResult, all_tokens,
};
use crate::bms::{model::Bms, prelude::*};

/// It processes `#VIDEOFILE`, `#MOVIE` and so on definitions and objects on `Seek` channel.
pub struct VideoProcessor<'a, P>(
    pub Rc<RefCell<Bms>>,
    #[cfg_attr(not(feature = "minor-command"), allow(dead_code))] pub &'a P,
);

impl<P: Prompter> TokenProcessor for VideoProcessor<'_, P> {
    fn process(&self, input: &mut &[&TokenWithRange<'_>]) -> TokenProcessorResult {
        all_tokens(input, |token| {
            Ok(match token {
                Token::Header { name, args } => self.on_header(name.as_ref(), args.as_ref()).err(),
                Token::Message {
                    track,
                    channel,
                    message,
                } => self.on_message(*track, *channel, message.as_ref()).err(),
                Token::NotACommand(_) => None,
            })
        })
    }
}

impl<P: Prompter> VideoProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "VIDEOFILE" => {
                if args.is_empty() {
                    return Err(ParseWarning::SyntaxError("expected video filename".into()));
                }
                self.0.borrow_mut().graphics.video_file = Some(Path::new(args).into());
            }
            "MOVIE" => {
                if args.is_empty() {
                    return Err(ParseWarning::SyntaxError("expected movie filename".into()));
                }
                self.0.borrow_mut().header.movie = Some(Path::new(args).into());
            }
            #[cfg(feature = "minor-command")]
            "VIDEOF/S" => {
                let frame_rate = Decimal::from_fraction(
                    GenericFraction::<BigUint>::from_str(args)
                        .map_err(|_| ParseWarning::SyntaxError("expected f64".into()))?,
                );
                self.0.borrow_mut().graphics.video_fs = Some(frame_rate);
            }
            #[cfg(feature = "minor-command")]
            "VIDEOCOLORS" => {
                let colors = args
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected u8".into()))?;
                self.0.borrow_mut().graphics.video_colors = Some(colors);
            }
            #[cfg(feature = "minor-command")]
            "VIDEODLY" => {
                let delay = Decimal::from_fraction(
                    GenericFraction::<BigUint>::from_str(args)
                        .map_err(|_| ParseWarning::SyntaxError("expected f64".into()))?,
                );
                self.0.borrow_mut().graphics.video_dly = Some(delay);
            }
            #[cfg(feature = "minor-command")]
            seek if seek.starts_with("SEEK") => {
                use fraction::GenericFraction;
                use num::BigUint;

                let id = &name["SEEK".len()..];
                let ms = Decimal::from_fraction(
                    GenericFraction::<BigUint>::from_str(args)
                        .map_err(|_| ParseWarning::SyntaxError("expected decimal".into()))?,
                );
                let id = ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id)?;

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
            _ => {}
        }
        Ok(())
    }

    fn on_message(&self, _track: Track, channel: Channel, _message: &str) -> Result<()> {
        match channel {
            #[cfg(feature = "minor-command")]
            Channel::Seek => {
                for (time, seek_id) in ids_from_message(
                    _track,
                    _message,
                    self.0.borrow().header.case_sensitive_obj_id,
                    |w| self.1.warn(w),
                ) {
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
