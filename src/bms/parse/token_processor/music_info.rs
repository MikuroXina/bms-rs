//! This module handles the tokens:
//!
//! - `#GENRE genre` - Genre of the music.
//! - `#TITLE title` - Title of the music.
//! - `#SUBTITLE subtitle` - Subtitle of the music.
//! - `#ARTIST artist` - Song author of the music,
//! - `#SUBARTIST sub_artist` - Song co-authors of the music,
//! - `#COMMENT comment` - Creation comment of the music.
//! - `#MAKER author` - Author of the score.
//! - `#PREVIEW path` - Path of the preview music file.

use std::path::Path;

use super::{ProcessContext, TokenProcessor};
use crate::bms::ParseErrorWithRange;
use crate::bms::{model::music_info::MusicInfo, parse::Result, prelude::*};

/// It processes music information headers such as `#GENRE`, `#TITLE` and so on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MusicInfoProcessor;

impl TokenProcessor for MusicInfoProcessor {
    type Output = MusicInfo;

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> core::result::Result<Self::Output, ParseErrorWithRange> {
        let mut music_info = MusicInfo::default();
        ctx.all_tokens(|token, _prompter| match token.content() {
            Token::Header { name, args } => Ok(self
                .on_header(name.as_ref(), args.as_ref(), &mut music_info)
                .map(|()| None)
                .unwrap_or_else(|warn| Some(warn.into_wrapper(token)))),
            Token::Message { .. } | Token::NotACommand(_) => Ok(None),
        })?;
        Ok(music_info)
    }
}

impl MusicInfoProcessor {
    fn on_header(self, name: &str, args: &str, music_info: &mut MusicInfo) -> Result<()> {
        if name.eq_ignore_ascii_case("GENRE") {
            music_info.genre = Some(args.to_string());
        }
        if name.eq_ignore_ascii_case("TITLE") {
            music_info.title = Some(args.to_string());
        }
        if name.eq_ignore_ascii_case("SUBTITLE") {
            music_info.subtitle = Some(args.to_string());
        }
        if name.eq_ignore_ascii_case("ARTIST") {
            music_info.artist = Some(args.to_string());
        }
        if name.eq_ignore_ascii_case("SUBARTIST") {
            music_info.sub_artist = Some(args.to_string());
        }
        if name.eq_ignore_ascii_case("COMMENT") {
            music_info
                .comment
                .get_or_insert_with(Vec::new)
                .push(args.to_string());
        }
        if name.eq_ignore_ascii_case("MAKER") {
            music_info.maker = Some(args.to_string());
        }
        if name.eq_ignore_ascii_case("PREVIEW") {
            music_info.preview_music = Some(Path::new(args).into());
        }
        Ok(())
    }
}
