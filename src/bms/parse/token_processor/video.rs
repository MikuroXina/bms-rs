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
use super::{super::prompt::Prompter, Result, TokenProcessor};
use crate::bms::{model::Bms, prelude::*};
use std::ops::ControlFlow;

/// It processes `#VIDEOFILE`, `#MOVIE` and so on definitions and objects on `Seek` channel.
pub struct VideoProcessor<'a, P>(
    pub Rc<RefCell<Bms>>,
    #[cfg_attr(not(feature = "minor-command"), allow(dead_code))] pub &'a P,
);

impl<P: Prompter> TokenProcessor for VideoProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> ControlFlow<Result<()>> {
        match name.to_ascii_uppercase().as_str() {
            "VIDEOFILE" => {
                if args.is_empty() {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected video filename".into(),
                    )));
                }
                self.0.borrow_mut().graphics.video_file = Some(Path::new(args).into());
                ControlFlow::Break(Ok(()))
            }
            "MOVIE" => {
                if args.is_empty() {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected movie filename".into(),
                    )));
                }
                self.0.borrow_mut().header.movie = Some(Path::new(args).into());
                ControlFlow::Break(Ok(()))
            }
            #[cfg(feature = "minor-command")]
            "VIDEOF/S" => {
                let frame_rate = match GenericFraction::<BigUint>::from_str(args) {
                    Ok(frac) => Decimal::from_fraction(frac),
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected f64".into(),
                        )));
                    }
                };
                self.0.borrow_mut().graphics.video_fs = Some(frame_rate);
                ControlFlow::Break(Ok(()))
            }
            #[cfg(feature = "minor-command")]
            "VIDEOCOLORS" => {
                let colors = match args.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected u8".into(),
                        )));
                    }
                };
                self.0.borrow_mut().graphics.video_colors = Some(colors);
                ControlFlow::Break(Ok(()))
            }
            #[cfg(feature = "minor-command")]
            "VIDEODLY" => {
                let delay = match GenericFraction::<BigUint>::from_str(args) {
                    Ok(frac) => Decimal::from_fraction(frac),
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected f64".into(),
                        )));
                    }
                };
                self.0.borrow_mut().graphics.video_dly = Some(delay);
                ControlFlow::Break(Ok(()))
            }
            #[cfg(feature = "minor-command")]
            seek if seek.starts_with("SEEK") => {
                use fraction::GenericFraction;
                use num::BigUint;

                let id = &name["SEEK".len()..];
                let ms = match GenericFraction::<BigUint>::from_str(args) {
                    Ok(frac) => Decimal::from_fraction(frac),
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected decimal".into(),
                        )));
                    }
                };
                let id = match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                    Ok(v) => v,
                    Err(e) => return ControlFlow::Break(Err(e)),
                };

                if let Some(older) = self.0.borrow_mut().others.seek_events.get_mut(&id) {
                    if let Err(e) = self
                        .1
                        .handle_def_duplication(DefDuplication::SeekEvent {
                            id,
                            older,
                            newer: &ms,
                        })
                        .apply_def(older, ms, id)
                    {
                        return ControlFlow::Break(Err(e));
                    }
                } else {
                    self.0.borrow_mut().others.seek_events.insert(id, ms);
                }
                ControlFlow::Break(Ok(()))
            }
            _ => ControlFlow::Continue(()),
        }
    }

    fn on_message(
        &self,
        _track: Track,
        channel: Channel,
        _message: &str,
    ) -> ControlFlow<Result<()>> {
        match channel {
            #[cfg(feature = "minor-command")]
            Channel::Seek => {
                for (time, seek_id) in ids_from_message(
                    _track,
                    _message,
                    self.0.borrow().header.case_sensitive_obj_id,
                    |w| self.1.warn(w),
                ) {
                    let position = match self.0.borrow().others.seek_events.get(&seek_id).cloned() {
                        Some(v) => v,
                        None => {
                            return ControlFlow::Break(Err(ParseWarning::UndefinedObject(seek_id)));
                        }
                    };
                    if let Err(e) = self
                        .0
                        .borrow_mut()
                        .notes
                        .push_seek_event(SeekObj { time, position }, self.1)
                    {
                        return ControlFlow::Break(Err(e));
                    }
                }
                return ControlFlow::Break(Ok(()));
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }
}
