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

use std::{path::Path, str::FromStr};

use super::{TokenProcessor, TokenProcessorOutput, all_tokens};
use crate::bms::{error::Result, model::metadata::Metadata, prelude::*};

/// It processes metadata headers such as `#PLAYER`, `#DIFFICULTY` and so on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MetadataProcessor;

impl TokenProcessor for MetadataProcessor {
    type Output = Metadata;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        _prompter: &P,
    ) -> TokenProcessorOutput<Self::Output> {
        let mut metadata = Metadata::default();
        let (res, warnings) = all_tokens(input, |token| {
            Ok(match token {
                Token::Header { name, args } => self
                    .on_header(name.as_ref(), args.as_ref(), &mut metadata)
                    .err(),
                Token::Message { .. } => None,
                Token::NotACommand(line) => self.on_comment(line, &mut metadata).err(),
            })
        });
        match res {
            Ok(()) => (Ok(metadata), warnings),
            Err(e) => (Err(e), warnings),
        }
    }
}

impl MetadataProcessor {
    fn on_header(&self, name: &str, args: &str, metadata: &mut Metadata) -> Result<()> {
        if name.eq_ignore_ascii_case("PLAYER") {
            metadata.player = Some(PlayerMode::from_str(args)?);
        }
        if name.eq_ignore_ascii_case("DIFFICULTY") {
            metadata.difficulty = Some(
                args.parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?,
            );
        }
        if name.eq_ignore_ascii_case("PLAYLEVEL") {
            metadata.play_level = Some(
                args.parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?,
            );
        }
        if name.eq_ignore_ascii_case("EMAIL") {
            metadata.email = Some(args.to_string());
        }
        if name.eq_ignore_ascii_case("URL") {
            metadata.url = Some(args.to_string());
        }
        if name.eq_ignore_ascii_case("PATH_WAV") {
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError("expected wav root path".into()));
            }
            metadata.wav_path_root = Some(Path::new(args).into());
        }
        if name.eq_ignore_ascii_case("DIVIDEPROP") {
            metadata.divide_prop = Some(args.to_string());
        }
        if name.eq_ignore_ascii_case("OCT/FP") {
            metadata.is_octave = true;
        }
        Ok(())
    }

    fn on_comment(&self, line: &str, metadata: &mut Metadata) -> Result<()> {
        let line = line.trim();
        if line.starts_with("%EMAIL") {
            metadata.email = Some(line.trim_start_matches("%EMAIL").trim().to_string());
        }
        if line.starts_with("%URL") {
            metadata.url = Some(line.trim_start_matches("%URL").trim().to_string());
        }
        Ok(())
    }
}
