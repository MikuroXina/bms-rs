//! Lexical analyzer of BMS format.
//!
//! Raw [String] == [lex] ==> [TokenStream] (in [BmsLexOutput]) == [parse] ==> [Bms] (in
//! BmsParseOutput)

mod command_impl;
mod cursor;
pub mod token;

use thiserror::Error;

use crate::{
    bms::{
        command::channel::{Channel, read_channel_beat},
        prelude::SourcePosMixin,
    },
    command::mixin::SourcePosMixinExt,
};

use self::{cursor::Cursor, token::TokenContent};

/// An error occurred when lexical analysis.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LexWarning {
    /// The token was expected but not found.
    #[error("expected {message}, but not found")]
    ExpectedToken {
        /// What the expected is.
        message: String,
    },
    /// The channel was not recognized.
    #[error("channel `{channel}` not recognized")]
    UnknownChannel {
        /// The channel that was not recognized.
        channel: String,
    },
    /// The object id was not recognized.
    #[error("object `{object}` not recognized")]
    UnknownObject {
        /// The object id that was not recognized.
        object: String,
    },
    /// Failed to convert a byte into a base-62 character `0-9A-Za-z`.
    #[error("expected id format is base 62 (`0-9A-Za-z`)")]
    OutOfBase62,
}

impl SourcePosMixinExt for LexWarning {}

/// type alias of core::result::Result<T, LexWarning>
pub(crate) type Result<T> = core::result::Result<T, LexWarning>;

/// Lex Parsing Results, includes tokens and warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BmsLexOutput<'a> {
    /// tokens
    pub tokens: Vec<SourcePosMixin<TokenContent<'a>>>,
    /// warnings
    pub lex_warnings: Vec<SourcePosMixin<LexWarning>>,
}

/// Analyzes and converts the BMS format text into [`TokenStream`].
pub fn parse_lex_tokens<'a>(source: &'a str) -> BmsLexOutput<'a> {
    parse_lex_tokens_with_channel_parser(source, &read_channel_beat)
}

/// Analyzes and converts the BMS format text into [`TokenStream`].
/// Use this function when you want to parse the BMS format text with a custom channel parser.
pub fn parse_lex_tokens_with_channel_parser<'a>(
    source: &'a str,
    channel_parser: &'a impl Fn(&str) -> Option<Channel>,
) -> BmsLexOutput<'a> {
    let mut cursor = Cursor::new(source);

    let mut tokens = vec![];
    let mut warnings = vec![];
    while !cursor.is_end() {
        match TokenContent::parse(&mut cursor, channel_parser) {
            Ok(content) => tokens.push(content.into_wrapper_manual(cursor.line(), cursor.col())),
            Err(warning) => warnings.push(warning.into_wrapper_manual(cursor.line(), cursor.col())),
        };
    }

    let case_sensitive = tokens
        .iter()
        .any(|token| matches!(token.content, TokenContent::Base62));
    if !case_sensitive {
        for token in &mut tokens {
            token.content.make_id_uppercase();
        }
    }
    BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, str::FromStr};

    use fraction::{GenericDecimal, GenericFraction};

    use crate::{
        bms::{
            command::{
                JudgeLevel, PlayerMode,
                channel::{Channel, NoteKind, PlayerSide},
                time::Track,
            },
            lex::{BmsLexOutput, parse_lex_tokens, token::TokenContent::*},
        },
        command::channel::Key,
    };

    #[test]
    fn simple() {
        const SRC: &str = r"
#PLAYER 1
#GENRE FUGA
#TITLE BAR(^^)
#ARTIST MikuroXina
#BPM 120
#PLAYLEVEL 6
#RANK 2
#BACKBMP boon.jpg

#WAV01 hoge.WAV
#WAV02 foo.WAV
#WAV03 bar.WAV

#00211:0303030303

#00211:0303000303

#00211:010101
#00211:00020202
";

        let BmsLexOutput {
            tokens,
            lex_warnings: warnings,
        } = parse_lex_tokens(SRC);

        assert_eq!(warnings, vec![]);
        assert_eq!(
            tokens.into_iter().map(|t| t.content).collect::<Vec<_>>(),
            vec![
                Player(PlayerMode::Single),
                Genre("FUGA"),
                Title("BAR(^^)"),
                Artist("MikuroXina"),
                Bpm(GenericDecimal::from_fraction(
                    GenericFraction::from_str("120").unwrap()
                )),
                PlayLevel(6),
                Rank(JudgeLevel::Normal),
                BackBmp(Path::new("boon.jpg")),
                Wav("01".try_into().unwrap(), Path::new("hoge.WAV")),
                Wav("02".try_into().unwrap(), Path::new("foo.WAV")),
                Wav("03".try_into().unwrap(), Path::new("bar.WAV")),
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        kind: NoteKind::Visible,
                        side: PlayerSide::Player1,
                        key: Key::Key1,
                    },
                    message: "0303030303".into(),
                },
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        kind: NoteKind::Visible,
                        side: PlayerSide::Player1,
                        key: Key::Key1,
                    },
                    message: "0303000303".into(),
                },
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        kind: NoteKind::Visible,
                        side: PlayerSide::Player1,
                        key: Key::Key1,
                    },
                    message: "010101".into(),
                },
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        kind: NoteKind::Visible,
                        side: PlayerSide::Player1,
                        key: Key::Key1,
                    },
                    message: "00020202".into(),
                },
            ]
        );
    }
}
