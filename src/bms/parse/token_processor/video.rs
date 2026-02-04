//! This module handles the tokens:
//!
//! - `#VIDEOFILE filename` / `#MOVIE filename` - The video file path played as BGA.
//! - `#VIDEOf/s n` - Specifies playing frame rate of the video BGA.
//! - `#VIDEOCOLORS n` - Definies color palette (sample size) of the video BGA.
//! - `#VIDEODLY n` - Defines the start frame of playing the video BGA.
//! - `#SEEK[00-ZZ] n` - It controls playing time of the video BGA. Obsolete.
//! - `#xxx:05` - Video seek channel. Obsolete.

use std::{cell::RefCell, path::Path, rc::Rc};

use strict_num_extended::FinF64;

use super::{super::prompt::Prompter, ProcessContext, TokenProcessor};
use crate::bms::ParseErrorWithRange;
use crate::{
    bms::{
        model::video::Video,
        parse::{ParseWarning, Result},
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

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> core::result::Result<Self::Output, ParseErrorWithRange> {
        let mut video = Video::default();
        ctx.all_tokens(|token, prompter| match token.content() {
            Token::Header { name, args } => Ok(self
                .on_header(name.as_ref(), args.as_ref(), prompter, &mut video)
                .err()
                .map(|warn| warn.into_wrapper(token))),
            Token::Message {
                track,
                channel,
                message,
            } => Ok(self
                .on_message(
                    *track,
                    *channel,
                    message.as_ref().into_wrapper(token),
                    prompter,
                    &mut video,
                )
                .err()
                .map(|warn| warn.into_wrapper(token))),
            Token::NotACommand(_) => Ok(None),
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
        if name.eq_ignore_ascii_case("VIDEOFILE") || name.eq_ignore_ascii_case("MOVIE") {
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError("expected video filename".into()));
            }
            video.video_file = Some(Path::new(args).into());
        }
        if name.eq_ignore_ascii_case("VIDEOF/S") {
            let frame_rate = args
                .parse::<f64>()
                .ok()
                .and_then(|v| FinF64::new(v).ok())
                .ok_or_else(|| ParseWarning::SyntaxError("expected f64".into()))?;
            video.video_fs = Some(frame_rate);
        }
        if name.eq_ignore_ascii_case("VIDEOCOLORS") {
            let colors = args
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected u8".into()))?;
            video.video_colors = Some(colors);
        }
        if name.eq_ignore_ascii_case("VIDEODLY") {
            let delay = args
                .parse::<f64>()
                .ok()
                .and_then(|v| FinF64::new(v).ok())
                .ok_or_else(|| ParseWarning::SyntaxError("expected f64".into()))?;
            video.video_dly = Some(delay);
        }
        if let Some(id) = name.strip_prefix_ignore_case("SEEK") {
            let ms = args
                .parse::<f64>()
                .ok()
                .and_then(|v| FinF64::new(v).ok())
                .ok_or_else(|| ParseWarning::SyntaxError("expected decimal".into()))?;
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
    ) -> core::result::Result<Vec<ParseWarningWithRange>, ParseWarning> {
        let mut warnings: Vec<ParseWarningWithRange> = Vec::new();
        if channel == Channel::Seek {
            use super::parse_obj_ids;

            let (pairs, w) = parse_obj_ids(track, &message, &self.case_sensitive_obj_id);
            warnings.extend(w);
            for (time, seek_id) in pairs {
                let position = video
                    .seek_defs
                    .get(&seek_id)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(seek_id))?;
                video.push_seek_event(SeekObj { time, position }, prompter)?;
            }
        }
        Ok(warnings)
    }
}
