use std::{cell::RefCell, path::Path, rc::Rc, str::FromStr};

use super::{super::prompt::Prompter, Result, TokenProcessor};
use crate::bms::{model::Bms, prelude::*};

/// It processes metadata headers such as `#PLAYER`, `#DIFFICULTY` and so on.
pub struct MetadataProcessor<'a, P, T>(Rc<RefCell<Bms<T>>>, &'a P);

impl<P: Prompter, T: KeyLayoutMapper> TokenProcessor for MetadataProcessor<'_, P, T> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        match name {
            "PLAYER" => self.0.borrow_mut().header.player = Some(PlayerMode::from_str(args)?),
            "DIFFICULTY" => {
                self.0.borrow_mut().header.difficulty = Some(
                    args.parse()
                        .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?,
                );
            }
            "PLAYLEVEL" => {
                self.0.borrow_mut().header.play_level = Some(
                    args.parse()
                        .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?,
                );
            }
            "EMAIL" | "%EMAIL" => self.0.borrow_mut().header.email = Some(args.to_string()),
            "URL" | "%URL" => self.0.borrow_mut().header.url = Some(args.to_string()),
            "PATH_WAV" => {
                if args.is_empty() {
                    return Err(ParseWarning::SyntaxError("expected wav root path".into()));
                }
                self.0.borrow_mut().notes.wav_path_root = Some(Path::new(args).into());
            }
            #[cfg(feature = "minor-command")]
            "DIVIDEPROP" => {
                self.0.borrow_mut().others.divide_prop = Some(args.to_string());
            }
            "CHARSET" => {
                // `#CHARSET` doesn't have a meaning because this library accepts only UTF-8.
            }
            _ => {}
        }
        Ok(())
    }

    fn on_message(&self, _: Track, _: Channel, _: &str) -> Result<()> {
        Ok(())
    }
}
