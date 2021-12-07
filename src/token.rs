use std::ffi::OsStr;

use crate::{command::*, cursor::Cursor, Result};

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
    pub(crate) fn parse(c: &mut Cursor<'a>) -> Result<Self> {
        let command = c
            .next_token()
            .ok_or_else(|| c.err_expected_token("command"))?;

        Ok(match command.to_uppercase().as_str() {
            "#PLAYER" => Self::Player(PlayerMode::from(c)?),
            "#GENRE" => Self::Genre(
                c.next_token()
                    .ok_or_else(|| c.err_expected_token("genre"))?,
            ),
            "#TITLE" => Self::Title(
                c.next_token()
                    .ok_or_else(|| c.err_expected_token("title"))?,
            ),
            "#ARTIST" => Self::Artist(
                c.next_token()
                    .ok_or_else(|| c.err_expected_token("artist"))?,
            ),
            "#BPM" => Self::Bpm(
                c.next_token()
                    .ok_or_else(|| c.err_expected_token("bpm"))?
                    .parse()
                    .map_err(|_| c.err_expected_token("integer"))?,
            ),
            "#PLAYLEVEL" => Self::PlayLevel(
                c.next_token()
                    .ok_or_else(|| c.err_expected_token("play level"))?
                    .parse()
                    .map_err(|_| c.err_expected_token("integer"))?,
            ),
            "#RANK" => Self::Rank(JudgeLevel::from(c)?),
            wav if wav.starts_with("#WAV") => {
                let id = command.trim_start_matches("#WAV");
                let filename = OsStr::new(
                    c.next_token()
                        .ok_or_else(|| c.err_expected_token("key audio filename"))?,
                );
                Self::Wav(WavId::from(id, c)?, filename)
            }
            bmp if bmp.starts_with("#BMP") => {
                let id = command.trim_start_matches("#BMP");
                let filename = OsStr::new(
                    c.next_token()
                        .ok_or_else(|| c.err_expected_token("bgi image filename"))?,
                );
                Self::Bgi(BgiId::from(id, c)?, filename)
            }
            message
                if message.starts_with('#')
                    && message.chars().nth(6) == Some(':')
                    && 9 <= message.len()
                    && message.len() % 2 == 1 =>
            {
                let track = command[1..4]
                    .parse()
                    .map_err(|_| c.err_expected_token("[000-999]"))?;
                let channel = &command[4..6];
                let message = &command[8..];
                Self::Message {
                    track: Track(track),
                    channel: Channel::from(channel, c)?,
                    message,
                }
            }
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
