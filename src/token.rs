use std::ffi::OsStr;

use crate::{command::*, cursor::Cursor, ParseError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token<'a> {
    Player(PlayerMode),
    Genre(&'a str),
    Title(&'a str),
    Artist(&'a str),
    Bpm(u8),
    MidiFile(&'a OsStr),
    PlayLevel(u8),
    Rank(JudgeLevel),
    VolWav(Volume),
    Wav(WavId, &'a OsStr),
    Bgi(BgiId, &'a OsStr),
    Message {
        track: Track,
        channel: Channel,
        message: &'a str,
    },
}

impl<'a> Token<'a> {
    pub(crate) fn parse(cursor: &mut Cursor<'a>) -> Result<Self> {
        let command = cursor
            .next_token()
            .ok_or_else(|| ParseError::ExpectedToken {
                line: cursor.line(),
                col: cursor.col(),
                message: "expected command but not found",
            })?;

        Ok(match command {
            "#PLAYER" => Self::Player(PlayerMode::from(cursor)?),
            _ => todo!(),
        })
    }
}

pub struct TokenStream<'a> {
    tokens: Vec<Token<'a>>,
}

impl<'a> TokenStream<'a> {
    pub(crate) fn from_tokens(tokens: Vec<Token<'a>>) -> Self {
        Self { tokens }
    }

    pub fn iter(&self) -> TokenStreamIter<'_, 'a> {
        TokenStreamIter {
            iter: self.tokens.iter(),
        }
    }
}

impl<'a> IntoIterator for TokenStream<'a> {
    type Item = Token<'a>;
    type IntoIter = <Vec<Token<'a>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.into_iter()
    }
}

pub struct TokenStreamIter<'t, 'a> {
    iter: std::slice::Iter<'t, Token<'a>>,
}

impl<'t, 'a> Iterator for TokenStreamIter<'t, 'a> {
    type Item = &'t Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}
