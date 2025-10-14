//! This module handles the tokens:
//!
//! - `#PLAYER [1-4]` - Play mode option, but supporting is unreliable.
//!   - 1: Single play.
//!   - 2: Couple play. Almost unsupported.
//!   - 3: Double play.
//!   - 4: Battle play. Deprecated.
//! - `#DIFFICULTY [1-5]` - Difficulty stage to sort scores of the same music.
//! - `#PLAYLEVEL n` - Difficulty number to show to the user.
//! - `#EMAIL email` / `%EMAIL email` - Email address of the author of the score.
//! - `#URL url` / `%URL url` - Distribution URL of the score.
//! - `#PATH_WAV path` - Base path of `#WAV`'s filenames for debug.
//! - `#DIVIDEPROP n` - Dividing resolution of playing. Obsolete.
//! - `#OCT/FP` - Octave mode option.

use std::{cell::RefCell, path::Path, rc::Rc, str::FromStr};

use super::{Result, TokenProcessor};
use crate::bms::{model::Bms, prelude::*};
use std::ops::ControlFlow;

/// It processes metadata headers such as `#PLAYER`, `#DIFFICULTY` and so on.
pub struct MetadataProcessor(pub Rc<RefCell<Bms>>);

impl TokenProcessor for MetadataProcessor {
    fn on_header(&self, name: &str, args: &str) -> ControlFlow<Result<()>> {
        match name.to_ascii_uppercase().as_str() {
            "PLAYER" => {
                let mode = match PlayerMode::from_str(args) {
                    Ok(v) => v,
                    Err(e) => return ControlFlow::Break(Err(e)),
                };
                self.0.borrow_mut().header.player = Some(mode);
            }
            "DIFFICULTY" => {
                let difficulty = match args.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                self.0.borrow_mut().header.difficulty = Some(difficulty);
            }
            "PLAYLEVEL" => {
                let level = match args.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                self.0.borrow_mut().header.play_level = Some(level);
            }
            "EMAIL" => self.0.borrow_mut().header.email = Some(args.to_string()),
            "URL" => self.0.borrow_mut().header.url = Some(args.to_string()),
            "PATH_WAV" => {
                if args.is_empty() {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected wav root path".into(),
                    )));
                }
                self.0.borrow_mut().notes.wav_path_root = Some(Path::new(args).into());
            }
            #[cfg(feature = "minor-command")]
            "DIVIDEPROP" => self.0.borrow_mut().others.divide_prop = Some(args.to_string()),
            #[cfg(feature = "minor-command")]
            "OCT/FP" => self.0.borrow_mut().others.is_octave = true,
            _ => {
                return ControlFlow::Continue(());
            }
        }
        ControlFlow::Break(Ok(()))
    }

    fn on_message(&self, _: Track, _: Channel, _: &str) -> ControlFlow<Result<()>> {
        ControlFlow::Continue(())
    }

    fn on_comment(&self, line: &str) -> ControlFlow<Result<()>> {
        let line = line.trim();
        if line.starts_with("%EMAIL") {
            self.0.borrow_mut().header.email =
                Some(line.trim_start_matches("%EMAIL").trim().to_string());
        }
        if line.starts_with("%URL") {
            self.0.borrow_mut().header.url =
                Some(line.trim_start_matches("%URL").trim().to_string());
        }
        ControlFlow::Continue(())
    }
}
