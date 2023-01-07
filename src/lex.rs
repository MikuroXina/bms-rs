//! Lexical analyzer of BMS format.

pub mod command;
mod cursor;
pub mod token;

use self::{
    cursor::Cursor,
    token::{Token, TokenStream},
};

/// An error occurred when lexical analysis.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum LexError {
    /// An unknown command detected.
    UnknownCommand {
        /// The line number of the command detected.
        line: usize,
        /// The column number of the command detected.
        col: usize,
    },
    /// The token was expected but not found.
    ExpectedToken {
        /// The line number of the token expected.
        line: usize,
        /// The column number of the token expected.
        col: usize,
        /// What the expected is.
        message: &'static str,
    },
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexError::UnknownCommand { line, col } => {
                write!(f, "unknown command found at line {}, col {}", line, col)
            }
            LexError::ExpectedToken { line, col, message } => write!(
                f,
                "expected {}, but not found at line {}, col {}",
                message, line, col
            ),
        }
    }
}

impl std::error::Error for LexError {}

/// An error occurred when lexical analyzing the BMS format file.
pub type Result<T> = std::result::Result<T, LexError>;

/// Analyzes and converts the BMS format text into [`TokenStream`].
pub fn parse(source: &str) -> Result<TokenStream> {
    let mut cursor = Cursor::new(source);

    let mut tokens = vec![];
    while !cursor.is_end() {
        tokens.push(Token::parse(&mut cursor)?);
    }
    Ok(TokenStream::from_tokens(tokens))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

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

        let ts = parse(SRC).expect("SRC must be parsed");

        let id1 = 1.try_into().unwrap();
        let id2 = 2.try_into().unwrap();
        let id3 = 3.try_into().unwrap();
        let tokens: Vec<_> = ts.into_iter().collect();
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
                Wav(id1, Path::new("hoge.WAV")),
                Wav(id2, Path::new("foo.WAV")),
                Wav(id3, Path::new("bar.WAV")),
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        kind: NoteKind::Visible,
                        is_player1: true,
                        key: Key::Key1,
                    },
                    message: "0303030303",
                },
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        kind: NoteKind::Visible,
                        is_player1: true,
                        key: Key::Key1,
                    },
                    message: "0303000303",
                },
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        kind: NoteKind::Visible,
                        is_player1: true,
                        key: Key::Key1,
                    },
                    message: "010101",
                },
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        kind: NoteKind::Visible,
                        is_player1: true,
                        key: Key::Key1,
                    },
                    message: "00020202",
                },
            ]
        );
    }
}
