//! This module handles the tokens:
//!
//! - `#VIDEOFILE filename` / `#MOVIE filename` - The video file path played as BGA.
//! - `#VIDEOf/s n` - Specifies playing frame rate of the video BGA.
//! - `#VIDEOCOLORS n` - Definies color palette (sample size) of the video BGA.
//! - `#VIDEODLY n` - Defines the start frame of playing the video BGA.
//! - `#SEEK[00-ZZ] n` - It controls playing time of the video BGA. Obsolete.
//! - `#xxx:05` - Video seek channel. Obsolete.

use std::str::FromStr;
use std::{cell::RefCell, path::Path, rc::Rc};

use fraction::GenericFraction;

use num::BigUint;

use super::{super::prompt::Prompter, TokenProcessor, TokenProcessorResult, all_tokens_with_range};
use crate::bms::{error::Result, model::video::Video, prelude::*};

/// It processes `#VIDEOFILE`, `#MOVIE` and so on definitions and objects on `Seek` channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoProcessor {
    case_sensitive_obj_id: Rc<RefCell<bool>>,
}

impl VideoProcessor {
    pub fn new(case_sensitive_obj_id: &Rc<RefCell<bool>>) -> Self {
        Self {
            case_sensitive_obj_id: Rc::clone(case_sensitive_obj_id),
        }
    }
}

impl TokenProcessor for VideoProcessor {
    type Output = Video;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> TokenProcessorResult<Self::Output> {
        let mut video = Video::default();
        all_tokens_with_range(input, prompter, |token| {
            Ok(match token.content() {
                Token::Header { name, args } => self
                    .on_header(name.as_ref(), args.as_ref(), prompter, &mut video)
                    .err(),
                Token::Message {
                    track,
                    channel,
                    message,
                } => self
                    .on_message(
                        *track,
                        *channel,
                        message.as_ref().into_wrapper(token),
                        prompter,
                        &mut video,
                    )
                    .err(),
                Token::NotACommand(_) => None,
            })
        })?;
        Ok(video)
    }
}

impl VideoProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        video: &mut Video,
    ) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "VIDEOFILE" | "MOVIE" => {
                if args.is_empty() {
                    return Err(ParseWarning::SyntaxError("expected video filename".into()));
                }
                video.video_file = Some(Path::new(args).into());
            }

            "VIDEOF/S" => {
                let frame_rate = Decimal::from_fraction(
                    GenericFraction::<BigUint>::from_str(args)
                        .map_err(|_| ParseWarning::SyntaxError("expected f64".into()))?,
                );
                video.video_fs = Some(frame_rate);
            }

            "VIDEOCOLORS" => {
                let colors = args
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected u8".into()))?;
                video.video_colors = Some(colors);
            }

            "VIDEODLY" => {
                let delay = Decimal::from_fraction(
                    GenericFraction::<BigUint>::from_str(args)
                        .map_err(|_| ParseWarning::SyntaxError("expected f64".into()))?,
                );
                video.video_dly = Some(delay);
            }

            seek if seek.starts_with("SEEK") => {
                use fraction::GenericFraction;
                use num::BigUint;

                let id = &name["SEEK".len()..];
                let ms = Decimal::from_fraction(
                    GenericFraction::<BigUint>::from_str(args)
                        .map_err(|_| ParseWarning::SyntaxError("expected decimal".into()))?,
                );
                let id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;

                if let Some(older) = video.seek_defs.get_mut(&id) {
                    prompter
                        .handle_def_duplication(DefDuplication::SeekEvent {
                            id,
                            older,
                            newer: &ms,
                        })
                        .apply_def(older, ms, id)?;
                } else {
                    video.seek_defs.insert(id, ms);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn on_message(
        &self,
        track: Track,
        channel: Channel,
        message: SourceRangeMixin<&str>,
        prompter: &impl Prompter,
        video: &mut Video,
    ) -> Result<()> {
        if let Channel::Seek = channel {
            use super::parse_obj_ids;

            for (time, seek_id) in
                parse_obj_ids(track, message, prompter, &self.case_sensitive_obj_id)
            {
                let position = video
                    .seek_defs
                    .get(&seek_id)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(seek_id))?;
                video.push_seek_event(SeekObj { time, position }, prompter)?;
            }
        }
        Ok(())
    }
}
