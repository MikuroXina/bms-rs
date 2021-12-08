pub mod command;
pub mod cursor;
pub mod token;

use self::{
    cursor::Cursor,
    token::{Token, TokenStream},
};

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum LexError {
    UnknownCommand {
        line: usize,
        col: usize,
    },
    ExpectedToken {
        line: usize,
        col: usize,
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

pub type Result<T> = std::result::Result<T, LexError>;

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
    use std::ffi::OsStr;

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

#WAV01 hoge.WAV
#WAV02 foo.WAV
#WAV03 bar.WAV

#00211:0303030303

#00211:0303000303

#00211:010101
#00211:00020202
";

        let ts = parse(SRC).expect("SRC must be parsed");

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
                Wav(WavId(1.try_into().unwrap()), OsStr::new("hoge.WAV")),
                Wav(WavId(2.try_into().unwrap()), OsStr::new("foo.WAV")),
                Wav(WavId(3.try_into().unwrap()), OsStr::new("bar.WAV")),
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
