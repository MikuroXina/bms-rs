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

use super::{super::prompt::Prompter, TokenProcessor, all_tokens_with_range};
use crate::{
    bms::{
        error::{ParseErrorWithRange, Result},
        model::video::Video,
        prelude::*,
    },
    util::StrExtension,
};

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
    ) -> (
        Self::Output,
        Vec<ParseWarningWithRange>,
        Vec<ParseErrorWithRange>,
    ) {
        let mut video = Video::default();
        let (_, warnings, errors) = all_tokens_with_range(input, prompter, |token| {
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
        });
        (video, warnings, errors)
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
        if name.eq_ignore_ascii_case("VIDEOFILE") || name.eq_ignore_ascii_case("MOVIE") {
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError("expected video filename".into()));
            }
            video.video_file = Some(Path::new(args).into());
        }
        if name.eq_ignore_ascii_case("VIDEOF/S") {
            let frame_rate = Decimal::from_fraction(
                GenericFraction::<BigUint>::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected f64".into()))?,
            );
            video.video_fs = Some(frame_rate);
        }
        if name.eq_ignore_ascii_case("VIDEOCOLORS") {
            let colors = args
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected u8".into()))?;
            video.video_colors = Some(colors);
        }
        if name.eq_ignore_ascii_case("VIDEODLY") {
            let delay = Decimal::from_fraction(
                GenericFraction::<BigUint>::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected f64".into()))?,
            );
            video.video_dly = Some(delay);
        }
        if let Some(id) = name.strip_prefix_ignore_case("SEEK") {
            use fraction::GenericFraction;
            use num::BigUint;

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
