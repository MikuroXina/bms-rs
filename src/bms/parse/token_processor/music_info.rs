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

use super::{TokenProcessor, TokenProcessorResult, all_tokens};
use crate::bms::{error::Result, model::music_info::MusicInfo, prelude::*};

/// It processes music information headers such as `#GENRE`, `#TITLE` and so on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MusicInfoProcessor;

impl TokenProcessor for MusicInfoProcessor {
    type Output = MusicInfo;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> TokenProcessorResult<Self::Output> {
        let mut music_info = MusicInfo::default();
        all_tokens(input, prompter, |token| {
            Ok(match token {
                Token::Header { name, args } => self
                    .on_header(name.as_ref(), args.as_ref(), &mut music_info)
                    .err(),
                Token::Message { .. } | Token::NotACommand(_) => None,
            })
        })?;
        Ok(music_info)
    }
}

impl MusicInfoProcessor {
    fn on_header(&self, name: &str, args: &str, music_info: &mut MusicInfo) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "GENRE" => music_info.genre = Some(args.to_string()),
            "TITLE" => music_info.title = Some(args.to_string()),
            "SUBTITLE" => music_info.subtitle = Some(args.to_string()),
            "ARTIST" => music_info.artist = Some(args.to_string()),
            "SUBARTIST" => music_info.sub_artist = Some(args.to_string()),
            "COMMENT" => music_info
                .comment
                .get_or_insert_with(Vec::new)
                .push(args.to_string()),
            "MAKER" => music_info.maker = Some(args.to_string()),
            "PREVIEW" => music_info.preview_music = Some(Path::new(args).into()),
            _ => {}
        }
        Ok(())
    }
}
