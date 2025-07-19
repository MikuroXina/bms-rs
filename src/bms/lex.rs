//! Lexical analyzer of BMS format.

pub mod command;
mod cursor;
pub mod token;

use std::borrow::Cow;

use thiserror::Error;

use crate::lex::command::channel::{read_channel_beat, Channel};

use self::{cursor::Cursor, token::Token};

/// A position in the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextPosition {
    /// The line number of the position.
    pub line: usize,
    /// The column number of the position.
    pub col: usize,
}

impl TextPosition {
    /// Creates a new [`TextPosition`].
    pub const fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

impl std::fmt::Display for TextPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "line {line}, col {col}",
            line = self.line,
            col = self.col
        )
    }
}

/// An error occurred when lexical analysis.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LexWarning {
    /// The token was expected but not found.
    #[error("expected {message}, but not found at {position}")]
    ExpectedToken {
        /// The position of the token expected.
        position: TextPosition,
        /// What the expected is.
        message: Cow<'static, str>,
    },
    /// The channel was not recognized.
    #[error("channel `{channel}` not recognized at {position}")]
    UnknownChannel {
        /// The channel that was not recognized.
        channel: Cow<'static, str>,
        /// The position of the channel that was not recognized.
        position: TextPosition,
    },
    /// The object was not recognized.
    #[error("object `{object}` not recognized at {position}")]
    UnknownObject {
        /// The object that was not recognized.
        object: Cow<'static, str>,
        /// The position of the object that was not recognized.
        position: TextPosition,
    },
    /// Failed to convert a byte into a base-62 character `0-9A-Za-z`.
    #[error("expected id format is base 62 (`0-9A-Za-z`)")]
    OutOfBase62,
}

/// type alias of core::result::Result<T, LexWarning>
pub(crate) type Result<T> = core::result::Result<T, LexWarning>;

/// Lex Parsing Results, includes tokens and warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BmsLexOutput<'a> {
    /// tokens
    pub tokens: Vec<Token<'a>>,
    /// warnings
    pub lex_warnings: Vec<LexWarning>,
}

/// Analyzes and converts the BMS format text into [`TokenStream`].
pub fn parse<'a>(source: &'a str) -> BmsLexOutput<'a> {
    parse_with_channel_parser(source, &read_channel_beat)
}

/// Analyzes and converts the BMS format text into [`TokenStream`].
/// Use this function when you want to parse the BMS format text with a custom channel parser.
pub fn parse_with_channel_parser<'a>(
    source: &'a str,
    channel_parser: &'a impl Fn(&str) -> Option<Channel>,
) -> BmsLexOutput<'a> {
    let mut cursor = Cursor::new(source);

    let mut tokens = vec![];
    let mut warnings = vec![];
    while !cursor.is_end() {
        match Token::parse(&mut cursor, &channel_parser) {
            Ok(token) => tokens.push(token),
            Err(warning) => warnings.push(warning),
        };
    }

    let case_sensitive = tokens.contains(&Token::Base62);
    if !case_sensitive {
        for token in &mut tokens {
            token.make_id_uppercase();
        }
    }
    BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::lex::BmsLexOutput;

    use super::{command::*, parse, token::Token::*};

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
        } = parse(SRC);

        assert_eq!(warnings, vec![]);
        assert_eq!(
            tokens,
            vec![
                Player(PlayerMode::Single),
                Genre("FUGA"),
                Title("BAR(^^)"),
                Artist("MikuroXina"),
                Bpm("120"),
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
