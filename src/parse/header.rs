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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bmp {
    pub file: PathBuf,
    pub transparent_color: Argb,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Header {
    pub player: Option<PlayerMode>,
    pub genre: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub artist: Option<String>,
    pub sub_artist: Option<String>,
    pub maker: Option<String>,
    pub comment: Option<Vec<String>>,
    pub email: Option<String>,
    pub url: Option<String>,
    pub options: Option<Vec<String>>,
    pub bpm: Option<f64>,
    pub play_level: Option<u8>,
    pub rank: Option<JudgeLevel>,
    pub difficulty: Option<u8>,
    pub total: Option<f64>,
    pub volume: Volume,
    pub ln_type: LnType,
    pub poor_bga_mode: PoorMode,
    pub back_bmp: Option<PathBuf>,
    pub stage_file: Option<PathBuf>,
    pub banner: Option<PathBuf>,
    pub is_octave: bool,
    pub midi_file: Option<PathBuf>,
    pub video_file: Option<PathBuf>,
    pub wav_path_root: Option<PathBuf>,
    pub wav_files: HashMap<ObjId, PathBuf>,
    pub poor_bmp: Option<PathBuf>,
    pub bmp_files: HashMap<ObjId, Bmp>,
    pub bpm_changes: HashMap<ObjId, f64>,
    pub texts: HashMap<ObjId, String>,
    pub change_options: HashMap<ObjId, String>,
}

impl Header {
    pub fn parse(&mut self, token: &Token) -> Result<()> {
        match *token {
            Token::Artist(artist) => self.artist = Some(artist.into()),
            Token::AtBga { .. } => todo!(),
            Token::Banner(file) => self.banner = Some(file.into()),
            Token::BackBmp(bmp) => self.back_bmp = Some(bmp.into()),
            Token::Bga { .. } => todo!(),
            Token::Bmp(id, path) => {
                if id.is_none() {
                    self.poor_bmp = Some(path.into());
                    return Ok(());
                }
                let id = id.unwrap();
                if self
                    .bmp_files
                    .insert(
                        id,
                        Bmp {
                            file: path.into(),
                            transparent_color: Argb::default(),
                        },
                    )
                    .is_some()
                {
                    eprintln!(
                        "duplicated bmp definition found: {:?} {:?}",
                        id,
                        path.display()
                    );
                }
            }
            Token::Bpm(bpm) => {
                if let Ok(parsed) = bpm.parse() {
                    if 0.0 < parsed {
                        self.bpm = Some(parsed);
                    } else {
                        eprintln!("not positive bpm found: {:?}", parsed);
                    }
                } else {
                    eprintln!("not number bpm found: {:?}", bpm);
                }
            }
            Token::BpmChange(id, bpm) => {
                let parsed: f64 = bpm
                    .parse()
                    .map_err(|_| ParseError::BpmParseError(bpm.into()))?;
                if parsed <= 0.0 || !parsed.is_finite() {
                    return Err(ParseError::BpmParseError(bpm.into()));
                }
                if self.bpm_changes.insert(id, parsed).is_some() {
                    eprintln!("duplicated bpm change definition found: {:?} {:?}", id, bpm);
                }
            }
            Token::ChangeOption(id, option) => {
                if self.change_options.insert(id, option.into()).is_some() {
                    eprintln!(
                        "duplicated change option definition found: {:?} {}",
                        id, option
                    );
                }
            }
            Token::Comment(comment) => self
                .comment
                .get_or_insert_with(Vec::new)
                .push(comment.into()),
            Token::Difficulty(diff) => self.difficulty = Some(diff),
            Token::Email(email) => self.email = Some(email.into()),
            Token::ExBmp(id, transparent_color, path) => {
                if self
                    .bmp_files
                    .insert(
                        id,
                        Bmp {
                            file: path.into(),
                            transparent_color,
                        },
                    )
                    .is_some()
                {
                    eprintln!(
                        "duplicated bmp definition found: {:?} {:?}",
                        id,
                        path.display()
                    );
                }
            }
            Token::ExRank(_, _) => todo!(),
            Token::ExWav(_, _, _) => todo!(),
            Token::Genre(genre) => self.genre = Some(genre.to_owned()),
            Token::LnTypeRdm => {
                self.ln_type = LnType::Rdm;
            }
            Token::LnTypeMgq => {
                self.ln_type = LnType::Mgq;
            }
            Token::Maker(maker) => self.maker = Some(maker.into()),
            Token::MidiFile(midi_file) => self.midi_file = Some(midi_file.into()),
            Token::OctFp => self.is_octave = true,
            Token::Option(option) => self
                .options
                .get_or_insert_with(Vec::new)
                .push(option.into()),
            Token::PathWav(wav_path_root) => self.wav_path_root = Some(wav_path_root.into()),
            Token::Player(player) => self.player = Some(player),
            Token::PlayLevel(play_level) => self.play_level = Some(play_level),
            Token::PoorBga(poor_bga_mode) => self.poor_bga_mode = poor_bga_mode,
            Token::Rank(rank) => self.rank = Some(rank),
            Token::StageFile(file) => self.stage_file = Some(file.into()),
            Token::SubArtist(sub_artist) => self.sub_artist = Some(sub_artist.into()),
            Token::SubTitle(subtitle) => self.subtitle = Some(subtitle.into()),
            Token::Text(id, text) => {
                if self.texts.insert(id, text.into()).is_some() {
                    eprintln!("duplicated text definition found: {:?} {}", id, text);
                }
            }
            Token::Title(title) => self.title = Some(title.into()),
            Token::Total(total) => {
                if let Ok(parsed) = total.parse() {
                    self.total = Some(parsed);
                } else {
                    eprintln!("not number total found: {:?}", total);
                }
            }
            Token::Url(url) => self.url = Some(url.into()),
            Token::VideoFile(video_file) => self.video_file = Some(video_file.into()),
            Token::VolWav(volume) => self.volume = volume,
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
