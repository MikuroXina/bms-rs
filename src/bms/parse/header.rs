//! Header information from parsed BMS file.

use std::{collections::HashMap, fmt::Debug, path::PathBuf};

use super::{
    ParseError, Result,
    prompt::{PromptHandler, PromptingDuplication},
};
use crate::lex::{command::*, token::Token};

/// A notation type about LN in the score. But you don't have to take care of how the notes are actually placed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LnType {
    /// The RDM type.
    Rdm,
    /// The MGQ type.
    Mgq,
}

impl Default for LnType {
    fn default() -> Self {
        Self::Rdm
    }
}

/// A background image/video data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bmp {
    /// The path to the image/video file. This is relative path from the BMS file.
    pub file: PathBuf,
    /// The color which should to be treated as transparent. It should be used only if `file` is an image.
    pub transparent_color: Argb,
}

/// A definition for #@BGA command.
#[derive(Debug, Clone, PartialEq)]
pub struct AtBgaDef {
    /// The object ID.
    pub id: ObjId,
    /// The source BMP object ID.
    pub source_bmp: ObjId,
    /// The top-left position for trimming.
    pub trim_top_left: (i16, i16),
    /// The size for trimming.
    pub trim_size: (u16, u16),
    /// The draw point position.
    pub draw_point: (i16, i16),
}

/// A definition for #BGA command.
#[derive(Debug, Clone, PartialEq)]
pub struct BgaDef {
    /// The object ID.
    pub id: ObjId,
    /// The source BMP object ID.
    pub source_bmp: ObjId,
    /// The top-left position for trimming.
    pub trim_top_left: (i16, i16),
    /// The bottom-right position for trimming.
    pub trim_bottom_right: (i16, i16),
    /// The draw point position.
    pub draw_point: (i16, i16),
}

/// A definition for #EXRANK command.
#[derive(Debug, Clone, PartialEq)]
pub struct ExRankDef {
    /// The object ID.
    pub id: ObjId,
    /// The judge level.
    pub judge_level: JudgeLevel,
}

/// A definition for #EXWAV command.
#[derive(Debug, Clone, PartialEq)]
pub struct ExWavDef {
    /// The object ID.
    pub id: ObjId,
    /// The parameters array.
    pub params: [String; 4],
    /// The file path.
    pub path: std::path::PathBuf,
}

/// A header parsed from [`TokenStream`](crate::lex::token::TokenStream).
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Header {
    /// The play style of the score.
    pub player: Option<PlayerMode>,
    /// The genre of the score.
    pub genre: Option<String>,
    /// The title of the score.
    pub title: Option<String>,
    /// The subtitle of the score.
    pub subtitle: Option<String>,
    /// The artist of the music in the score.
    pub artist: Option<String>,
    /// The co-artist of the music in the score.
    pub sub_artist: Option<String>,
    /// Who placed the notes into the score.
    pub maker: Option<String>,
    /// The text messages of the score. It may be closed with double quotes.
    pub comment: Option<Vec<String>>,
    /// The email address of the author.
    pub email: Option<String>,
    /// The url of the author.
    pub url: Option<String>,
    /// The message for overriding options of some BMS player.
    pub options: Option<Vec<String>>,
    /// The initial BPM of the score.
    pub bpm: Option<f64>,
    /// The play level of the score.
    pub play_level: Option<u8>,
    /// The judgement level of the score.
    pub rank: Option<JudgeLevel>,
    /// The difficulty of the score.
    pub difficulty: Option<u8>,
    /// The total gauge percentage when all notes is got as PERFECT.
    pub total: Option<f64>,
    /// The volume of the score.
    pub volume: Volume,
    /// The LN notation type of the score.
    pub ln_type: LnType,
    /// The display mode for background image/video.
    pub poor_bga_mode: PoorMode,
    /// The path of background image, which is shown while playing the score.
    pub back_bmp: Option<PathBuf>,
    /// The path of splash screen image, which is shown before playing the score.
    pub stage_file: Option<PathBuf>,
    /// The path of banner image.
    pub banner: Option<PathBuf>,
    /// Whether the score is the octave mode.
    pub is_octave: bool,
    /// The path of MIDI file, which is played as BGM while playing the score.
    pub midi_file: Option<PathBuf>,
    /// The path of the background video. The video should be started the playing from the section 000.
    pub video_file: Option<PathBuf>,
    /// The path to override the base path of the WAV file path.
    pub wav_path_root: Option<PathBuf>,
    /// The WAV file paths corresponding to the id of the note object.
    pub wav_files: HashMap<ObjId, PathBuf>,
    /// The path of image, which is shown when the player got POOR.
    pub poor_bmp: Option<PathBuf>,
    /// The BMP file paths corresponding to the id of the background image/video object.
    pub bmp_files: HashMap<ObjId, Bmp>,
    /// The BPMs corresponding to the id of the BPM change object.
    pub bpm_changes: HashMap<ObjId, f64>,
    /// The scrolling factors corresponding to the id of the scroll speed change object.
    pub scrolling_factor_changes: HashMap<ObjId, f64>,
    /// The spacing factors corresponding to the id of the spacing change object.
    pub spacing_factor_changes: HashMap<ObjId, f64>,
    /// The texts corresponding to the id of the text object.
    pub texts: HashMap<ObjId, String>,
    /// The option messages corresponding to the id of the change option object.
    pub change_options: HashMap<ObjId, String>,
    /// Stop lengths by stop object id.
    pub stops: HashMap<ObjId, u32>,
    /// Storage for #@BGA definitions
    pub atbga_defs: HashMap<ObjId, AtBgaDef>,
    /// Storage for #BGA definitions
    pub bga_defs: HashMap<ObjId, BgaDef>,
    /// Storage for #EXRANK definitions
    pub exrank_defs: HashMap<ObjId, ExRankDef>,
    /// Storage for #EXWAV definitions
    pub exwav_defs: HashMap<ObjId, ExWavDef>,
}

impl Header {
    pub(crate) fn parse(
        &mut self,
        token: &Token,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match *token {
            Token::Artist(artist) => self.artist = Some(artist.into()),
            Token::AtBga {
                id,
                source_bmp,
                trim_top_left,
                trim_size,
                draw_point,
            } => {
                self.atbga_defs.insert(
                    id,
                    AtBgaDef {
                        id,
                        source_bmp,
                        trim_top_left,
                        trim_size,
                        draw_point,
                    },
                );
            }
            Token::Banner(file) => self.banner = Some(file.into()),
            Token::BackBmp(bmp) => self.back_bmp = Some(bmp.into()),
            Token::Bga {
                id,
                source_bmp,
                trim_top_left,
                trim_bottom_right,
                draw_point,
            } => {
                self.bga_defs.insert(
                    id,
                    BgaDef {
                        id,
                        source_bmp,
                        trim_top_left,
                        trim_bottom_right,
                        draw_point,
                    },
                );
            }
            Token::Bmp(id, path) => {
                if id.is_none() {
                    self.poor_bmp = Some(path.into());
                    return Ok(());
                }
                let id = id.unwrap();
                let to_insert = Bmp {
                    file: path.into(),
                    transparent_color: Argb::default(),
                };
                if let Some(older) = self.bmp_files.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Bmp {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.bmp_files.insert(id, to_insert);
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
                if let Some(older) = self.bpm_changes.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::BpmChange {
                            id,
                            older: *older,
                            newer: parsed,
                        })
                        .apply(older, parsed)?;
                } else {
                    self.bpm_changes.insert(id, parsed);
                }
            }
            Token::ChangeOption(id, option) => {
                self.change_options.insert(id, option.into());
            }
            Token::Comment(comment) => self
                .comment
                .get_or_insert_with(Vec::new)
                .push(comment.into()),
            Token::Difficulty(diff) => self.difficulty = Some(diff),
            Token::Email(email) => self.email = Some(email.into()),
            Token::ExBmp(id, transparent_color, path) => {
                let to_insert = Bmp {
                    file: path.into(),
                    transparent_color,
                };
                if let Some(older) = self.bmp_files.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Bmp {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.bmp_files.insert(id, to_insert);
                }
            }
            Token::ExRank(id, judge_level) => {
                self.exrank_defs.insert(id, ExRankDef { id, judge_level });
            }
            Token::ExWav(id, params, path) => {
                self.exwav_defs.insert(
                    id,
                    ExWavDef {
                        id,
                        params: [
                            params[0].to_string(),
                            params[1].to_string(),
                            params[2].to_string(),
                            params[3].to_string(),
                        ],
                        path: path.into(),
                    },
                );
            }
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
            Token::Scroll(id, factor) => {
                let parsed: f64 = factor
                    .parse()
                    .map_err(|_| ParseError::BpmParseError(factor.into()))?;
                if parsed <= 0.0 || !parsed.is_finite() {
                    return Err(ParseError::BpmParseError(factor.into()));
                }
                if let Some(older) = self.scrolling_factor_changes.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::ScrollingFactorChange {
                            id,
                            older: *older,
                            newer: parsed,
                        })
                        .apply(older, parsed)?;
                } else {
                    self.scrolling_factor_changes.insert(id, parsed);
                }
            }
            Token::Speed(id, factor) => {
                let parsed: f64 = factor
                    .parse()
                    .map_err(|_| ParseError::BpmParseError(factor.into()))?;
                if parsed <= 0.0 || !parsed.is_finite() {
                    return Err(ParseError::BpmParseError(factor.into()));
                }
                if let Some(older) = self.spacing_factor_changes.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::SpacingFactorChange {
                            id,
                            older: *older,
                            newer: parsed,
                        })
                        .apply(older, parsed)?;
                } else {
                    self.spacing_factor_changes.insert(id, parsed);
                }
            }
            Token::StageFile(file) => self.stage_file = Some(file.into()),
            Token::Stop(id, len) => {
                self.stops
                    .entry(id)
                    .and_modify(|current_len| *current_len += len)
                    .or_insert(len);
            }
            Token::SubArtist(sub_artist) => self.sub_artist = Some(sub_artist.into()),
            Token::SubTitle(subtitle) => self.subtitle = Some(subtitle.into()),
            Token::Text(id, text) => {
                self.texts.insert(id, text.into());
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
                if let Some(older) = self.wav_files.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Wav {
                            id,
                            older,
                            newer: path,
                        })
                        .apply(older, path.into())?;
                } else {
                    self.wav_files.insert(id, path.into());
                }
            }
            Token::Base62
            | Token::Case(_)
            | Token::Def
            | Token::Else
            | Token::ElseIf(_)
            | Token::EndIf
            | Token::EndRandom
            | Token::EndSwitch
            | Token::If(_)
            | Token::LnObj(_)
            | Token::NotACommand(_)
            | Token::Random(_)
            | Token::SetRandom(_)
            | Token::SetSwitch(_)
            | Token::Skip
            | Token::Switch(_)
            | Token::ExtendedMessage { .. }
            | Token::Message { .. } => {
                // These Token should not be handled in Header::parse.
            }
        }
        Ok(())
    }
}
