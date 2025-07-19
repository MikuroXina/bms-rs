//! Lexical analyzer of BMS format.

pub mod command;
mod cursor;
pub mod token;

use thiserror::Error;

use self::{cursor::Cursor, token::Token};

/// An error occurred when lexical analysis.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LexWarning {
    /// An unknown command detected.
    #[error("unknown command found at line {line}, col {col}")]
    UnknownCommand {
        /// The line number of the command detected.
        line: usize,
        /// The column number of the command detected.
        col: usize,
    },
    /// The token was expected but not found.
    #[error("expected {message}, but not found at line {line}, col {col}")]
    ExpectedToken {
        /// The line number of the token expected.
        line: usize,
        /// The column number of the token expected.
        col: usize,
        /// What the expected is.
        message: &'static str,
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
pub fn parse(source: &str) -> BmsLexOutput {
    let mut cursor = Cursor::new(source);

    let mut tokens = vec![];
    let mut warnings = vec![];
    while !cursor.is_end() {
        match Token::parse(&mut cursor) {
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
                        is_player1: true,
                        key: Key::Key1,
                    },
                    message: "0303030303".into(),
                },
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        kind: NoteKind::Visible,
                        is_player1: true,
                        key: Key::Key1,
                    },
                    message: "0303000303".into(),
                },
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        kind: NoteKind::Visible,
                        is_player1: true,
                        key: Key::Key1,
                    },
                    message: "010101".into(),
                },
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        kind: NoteKind::Visible,
                        is_player1: true,
                        key: Key::Key1,
                    },
                    message: "00020202".into(),
                },
            ]
        );
    }
}
