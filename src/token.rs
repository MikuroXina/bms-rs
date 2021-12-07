use std::ffi::OsStr;

use crate::command::*;

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
    Message(Message<'a>),
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

pub struct TokenStreamIter<'t, 'a> {
    iter: std::slice::Iter<'t, Token<'a>>,
}

impl<'t, 'a> Iterator for TokenStreamIter<'t, 'a> {
    type Item = &'t Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}
