use std::{cell::RefCell, path::Path, rc::Rc};

use super::{super::prompt::Prompter, Result, TokenProcessor};
use crate::bms::{model::Bms, prelude::*};

/// It processes music information headers such as `#GENRE`, `#TITLE` and so on.
pub struct MusicInfoProcessor<'a, P, T>(Rc<RefCell<Bms<T>>>, &'a P);

impl<P: Prompter, T: KeyLayoutMapper> TokenProcessor for MusicInfoProcessor<'_, P, T> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        match name {
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

    fn on_message(&self, _: Track, _: Channel, _: &str) -> Result<()> {
        Ok(())
    }
}
