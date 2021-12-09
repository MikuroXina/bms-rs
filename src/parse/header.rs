use std::{collections::HashMap, fmt::Debug, path::PathBuf};

use super::{ParseError, Result};
use crate::lex::{command::*, token::Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LnType {
    Rdm,
    Mgq,
}

impl Default for LnType {
    fn default() -> Self {
        Self::Rdm
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Header {
    pub player: Option<PlayerMode>,
    pub genre: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub bpm: Option<f64>,
    pub play_level: Option<u8>,
    pub rank: Option<JudgeLevel>,
    pub difficulty: Option<u8>,
    pub total: Option<f64>,
    pub ln_type: LnType,
    pub wav_files: HashMap<ObjId, PathBuf>,
    pub bmp_files: HashMap<ObjId, PathBuf>,
    pub bpm_changes: HashMap<ObjId, f64>,
}

impl Header {
    pub fn parse(&mut self, token: &Token) -> Result<()> {
        match *token {
            Token::Artist(artist) => self.artist = Some(artist.into()),
            Token::AtBga {
                id,
                source_bmp,
                trim_top_left,
                trim_size,
                draw_point,
            } => todo!(),
            Token::Banner(_) => todo!(),
            Token::BackBmp(_) => todo!(),
            Token::Bga {
                id,
                source_bmp,
                trim_top_left,
                trim_bottom_right,
                draw_point,
            } => todo!(),
            Token::Bmp(id, path) => {
                if self.bmp_files.insert(id, path.into()).is_some() {
                    eprintln!(
                        "duplicated bmp definition found: {:?} {:?}",
                        id,
                        path.display()
                    );
                }
            }
            Token::Bpm(bpm) => {
                if let Ok(parsed) = bpm.parse() {
                    self.bpm = Some(parsed);
                } else {
                    eprintln!("not number bpm found: {:?}", bpm);
                }
            }
            Token::BpmChange(id, bpm) => {
                if self
                    .bpm_changes
                    .insert(
                        id,
                        bpm.parse()
                            .map_err(|_| ParseError::BpmParseError(bpm.into()))?,
                    )
                    .is_some()
                {
                    eprintln!("duplicated bpm change definition found: {:?} {:?}", id, bpm);
                }
            }
            Token::ChangeOption(_, _) => todo!(),
            Token::Comment(_) => todo!(),
            Token::Def => todo!(),
            Token::Difficulty(diff) => self.difficulty = Some(diff),
            Token::Email(_) => todo!(),
            Token::ExBmp(_, _, _) => todo!(),
            Token::ExRank(_, _) => todo!(),
            Token::ExWav(_, _, _) => todo!(),
            Token::Genre(genre) => self.genre = Some(genre.to_owned()),
            Token::LnObj(_) => todo!(),
            Token::LnTypeRdm => {
                self.ln_type = LnType::Rdm;
            }
            Token::LnTypeMgq => {
                self.ln_type = LnType::Mgq;
            }
            Token::Maker(_) => todo!(),
            Token::MidiFile(_) => todo!(),
            Token::OctFp => todo!(),
            Token::Option(_) => todo!(),
            Token::PathWav(_) => todo!(),
            Token::Player(player) => self.player = Some(player),
            Token::PlayLevel(play_level) => self.play_level = Some(play_level),
            Token::PoorBga(_) => todo!(),
            Token::Random(_) => todo!(),
            Token::Rank(rank) => self.rank = Some(rank),
            Token::SetRandom(_) => todo!(),
            Token::SetSwitch(_) => todo!(),
            Token::Skip => todo!(),
            Token::StageFile(_) => todo!(),
            Token::SubArtist(_) => todo!(),
            Token::SubTitle(_) => todo!(),
            Token::Switch(_) => todo!(),
            Token::Text(_, _) => todo!(),
            Token::Title(title) => self.title = Some(title.into()),
            Token::Total(total) => {
                if let Ok(parsed) = total.parse() {
                    self.total = Some(parsed);
                } else {
                    eprintln!("not number total found: {:?}", total);
                }
            }
            Token::Url(_) => todo!(),
            Token::VideoFile(_) => todo!(),
            Token::VolWav(_) => todo!(),
            Token::Wav(id, path) => {
                if self.wav_files.insert(id, path.into()).is_some() {
                    eprintln!(
                        "duplicated wav definition found: {:?} {:?}",
                        id,
                        path.display()
                    );
                }
            }
            _ => {}
        }
        Ok(())
    }
}
