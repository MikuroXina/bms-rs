use std::path::Path;

use super::{command::*, cursor::Cursor, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token<'a> {
    Artist(&'a str),
    AtBga {
        id: ObjId,
        source_bmp: ObjId,
        trim_top_left: (i16, i16),
        trim_size: (u16, u16),
        draw_point: (i16, i16),
    },
    Banner(&'a Path),
    BackBmp(&'a Path),
    BaseBpm(&'a str),
    Bga {
        id: ObjId,
        source_bmp: ObjId,
        trim_top_left: (i16, i16),
        trim_bottom_right: (i16, i16),
        draw_point: (i16, i16),
    },
    Bmp(ObjId, &'a Path),
    Bpm(&'a str),
    BpmChange(ObjId, &'a str),
    Case(u32),
    ChangeOption(ObjId, &'a str),
    Comment(&'a str),
    Def,
    Difficulty(u8),
    Else,
    ElseIf(u32),
    Email(&'a str),
    EndIf,
    EndRandom,
    EndSwitch,
    ExBmp(ObjId, Argb, &'a Path),
    ExRank(ObjId, JudgeLevel),
    ExWav(ObjId, [&'a str; 4], &'a Path),
    Genre(&'a str),
    If(u32),
    LnObj(ObjId),
    LnTypeRdm,
    LnTypeMgq,
    Maker(&'a str),
    Message {
        track: Track,
        channel: Channel,
        message: Vec<Option<ObjId>>,
    },
    MidiFile(&'a Path),
    OctFp,
    Option(&'a str),
    PathWav(&'a Path),
    Player(PlayerMode),
    PlayLevel(u8),
    PoorBga(PoorMode),
    Random(u32),
    Rank(JudgeLevel),
    SetRandom(u32),
    SetSwitch(u32),
    Skip,
    StageFile(&'a Path),
    SubArtist(&'a str),
    SubTitle(&'a str),
    Switch(u32),
    Text(ObjId, &'a str),
    Title(&'a str),
    Total(&'a str),
    Url(&'a str),
    VideoFile(&'a Path),
    VolWav(Volume),
    Wav(ObjId, &'a Path),
}

impl<'a> Token<'a> {
    pub(crate) fn parse(c: &mut Cursor<'a>) -> Result<Self> {
        loop {
            let command = c
                .next_token()
                .ok_or_else(|| c.err_expected_token("command"))?;

            break Ok(match command.to_uppercase().as_str() {
                "#PLAYER" => Self::Player(PlayerMode::from(c)?),
                "#GENRE" => Self::Genre(c.next_line_remaining()),
                "#TITLE" => Self::Title(c.next_line_remaining()),
                "#SUBTITLE" => Self::Title(c.next_line_remaining()),
                "#ARTIST" => Self::Artist(c.next_line_remaining()),
                "#SUBARTIST" => Self::SubArtist(c.next_line_remaining()),
                "#DIFFICULTY" => Self::Difficulty(
                    c.next_token()
                        .ok_or_else(|| c.err_expected_token("difficulty"))?
                        .parse()
                        .map_err(|_| c.err_expected_token("integer"))?,
                ),
                "#STAEGFILE" => Self::StageFile(
                    c.next_token()
                        .map(Path::new)
                        .ok_or_else(|| c.err_expected_token("stage filename"))?,
                ),
                "#BANNER" => Self::Banner(
                    c.next_token()
                        .map(Path::new)
                        .ok_or_else(|| c.err_expected_token("banner filename"))?,
                ),
                "#TOTAL" => Self::Total(
                    c.next_token()
                        .ok_or_else(|| c.err_expected_token("gauge increase rate"))?,
                ),
                "#BPM" => Self::Bpm(c.next_token().ok_or_else(|| c.err_expected_token("bpm"))?),
                "#PLAYLEVEL" => Self::PlayLevel(
                    c.next_token()
                        .ok_or_else(|| c.err_expected_token("play level"))?
                        .parse()
                        .map_err(|_| c.err_expected_token("integer"))?,
                ),
                "#RANK" => Self::Rank(JudgeLevel::from(c)?),
                "#LNTYPE" => {
                    if c.next_token() == Some("2") {
                        Self::LnTypeMgq
                    } else {
                        Self::LnTypeRdm
                    }
                }
                "#RANDOM" => {
                    let rand_max = c
                        .next_token()
                        .ok_or_else(|| c.err_expected_token("random max"))?
                        .parse()
                        .map_err(|_| c.err_expected_token("integer"))?;
                    Self::Random(rand_max)
                }
                "#ENDRANDOM" => Self::EndRandom,
                "#IF" => {
                    let rand_target = c
                        .next_token()
                        .ok_or_else(|| c.err_expected_token("random target"))?
                        .parse()
                        .map_err(|_| c.err_expected_token("integer"))?;
                    Self::If(rand_target)
                }
                "#ENDIF" => Self::EndIf,
                wav if wav.starts_with("#WAV") => {
                    let id = command.trim_start_matches("#WAV");
                    let filename = Path::new(
                        c.next_token()
                            .ok_or_else(|| c.err_expected_token("key audio filename"))?,
                    );
                    Self::Wav(ObjId::from(id, c)?, filename)
                }
                bmp if bmp.starts_with("#BMP") => {
                    let id = command.trim_start_matches("#BMP");
                    let filename = Path::new(
                        c.next_token()
                            .ok_or_else(|| c.err_expected_token("bgi image filename"))?,
                    );
                    Self::Bmp(ObjId::from(id, c)?, filename)
                }
                bpm if bpm.starts_with("#BPM") => {
                    let id = command.trim_start_matches("#BPM");
                    let bpm = c.next_token().ok_or_else(|| c.err_expected_token("bpm"))?;
                    Self::BpmChange(ObjId::from(id, c)?, bpm)
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

                    let message_ids = &command[7..];
                    let messages_len = message_ids.len() / 2;
                    let mut message = Vec::with_capacity(messages_len);
                    for i in (0..messages_len).map(|n| 2 * n) {
                        if &message_ids[i..i + 2] == "00" {
                            message.push(None);
                        } else {
                            message.push(Some(ObjId::from(&message_ids[i..i + 2], c)?));
                        }
                    }
                    Self::Message {
                        track: Track(track),
                        channel: Channel::from(channel, c)?,
                        message,
                    }
                }
                comment if !comment.starts_with('#') => {
                    c.next_line_remaining();
                    continue;
                }
                unknown => {
                    eprintln!("unknown command found: {:?}", unknown);
                    todo!();
                }
            });
        }
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

#[derive(Debug)]
pub struct TokenStreamIter<'t, 'a> {
    iter: std::slice::Iter<'t, Token<'a>>,
}

impl<'t, 'a> Iterator for TokenStreamIter<'t, 'a> {
    type Item = &'t Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}
