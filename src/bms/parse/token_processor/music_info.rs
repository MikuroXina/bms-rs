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

use std::{cell::RefCell, path::Path, rc::Rc};

use super::{super::Result, TokenProcessor, TokenProcessorResult, all_tokens};
use crate::bms::{model::Bms, prelude::*};

/// It processes music information headers such as `#GENRE`, `#TITLE` and so on.
pub struct MusicInfoProcessor(pub Rc<RefCell<Bms>>);

impl TokenProcessor for MusicInfoProcessor {
    fn process(&self, input: &mut &[TokenWithRange<'_>]) -> TokenProcessorResult {
        all_tokens(input, |token| {
            Ok(match token {
                Token::Header { name, args } => self.on_header(name.as_ref(), args.as_ref()).err(),
                Token::Message { .. } | Token::NotACommand(_) => None,
            })
        })
    }
}

impl MusicInfoProcessor {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "GENRE" => self.0.borrow_mut().header.genre = Some(args.to_string()),
            "TITLE" => self.0.borrow_mut().header.title = Some(args.to_string()),
            "SUBTITLE" => self.0.borrow_mut().header.subtitle = Some(args.to_string()),
            "ARTIST" => self.0.borrow_mut().header.artist = Some(args.to_string()),
            "SUBARTIST" => self.0.borrow_mut().header.sub_artist = Some(args.to_string()),
            "COMMENT" => self
                .0
                .borrow_mut()
                .header
                .comment
                .get_or_insert_with(Vec::new)
                .push(args.to_string()),
            "MAKER" => self.0.borrow_mut().header.maker = Some(args.to_string()),
            "PREVIEW" => self.0.borrow_mut().header.preview_music = Some(Path::new(args).into()),
            _ => {}
        }
        Ok(())
    }
}
