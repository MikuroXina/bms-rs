//! Header information from parsed BMS file.

use std::{collections::HashMap, fmt::Debug, path::PathBuf};

use super::{
    Result,
    prompt::{PromptHandler, PromptingDuplication},
};
use crate::lex::{command::*, token::Token};
use fraction::GenericFraction;
use num::{BigUint, ToPrimitive};

/// A 2D point in pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PixelPoint {
    /// X coordinate in pixels.
    pub x: i16,
    /// Y coordinate in pixels.
    pub y: i16,
}

impl PixelPoint {
    /// Creates a new pixel point.
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

impl From<(i16, i16)> for PixelPoint {
    fn from((x, y): (i16, i16)) -> Self {
        Self { x, y }
    }
}

impl From<PixelPoint> for (i16, i16) {
    fn from(point: PixelPoint) -> Self {
        (point.x, point.y)
    }
}

/// A 2D size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PixelSize {
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
}

impl PixelSize {
    /// Creates a new pixel size.
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

impl From<(u16, u16)> for PixelSize {
    fn from((width, height): (u16, u16)) -> Self {
        Self { width, height }
    }
}

impl From<PixelSize> for (u16, u16) {
    fn from(size: PixelSize) -> Self {
        (size.width, size.height)
    }
}

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AtBgaDef {
    /// The object ID.
    pub id: ObjId,
    /// The source BMP object ID.
    pub source_bmp: ObjId,
    /// The top-left position for trimming in pixels.
    pub trim_top_left: PixelPoint,
    /// The size for trimming in pixels.
    pub trim_size: PixelSize,
    /// The draw point position in pixels.
    pub draw_point: PixelPoint,
}

/// A definition for #BGA command.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BgaDef {
    /// The object ID.
    pub id: ObjId,
    /// The source BMP object ID.
    pub source_bmp: ObjId,
    /// The top-left position for trimming in pixels.
    pub trim_top_left: PixelPoint,
    /// The bottom-right position for trimming in pixels.
    pub trim_bottom_right: PixelPoint,
    /// The draw point position in pixels.
    pub draw_point: PixelPoint,
}

/// A definition for #EXRANK command.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExRankDef {
    /// The object ID.
    pub id: ObjId,
    /// The judge level.
    pub judge_level: JudgeLevel,
}

/// A definition for #EXWAV command.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExWavDef {
    /// The object ID.
    pub id: ObjId,
    /// The pan of the sound. Also called volume balance.
    /// Range: [-10000, 10000]. -10000 is leftmost, 10000 is rightmost.
    /// Default: 0.
    pub pan: ExWavPan,
    /// The volume of the sound.
    /// Range: [-10000, 0]. -10000 is 0%, 0 is 100%.
    /// Default: 0.
    pub volume: ExWavVolume,
    /// The frequency of the sound. Unit: Hz.
    /// Range: [100, 100000].
    /// Default: None.
    pub frequency: Option<ExWavFrequency>,
    /// The file path.
    pub path: std::path::PathBuf,
}

/// A header parsed from [`TokenStream`](crate::lex::token::TokenStream).
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Header {
    /// The play style of the score. #PLAYER
    pub player: Option<PlayerMode>,
    /// The genre of the score. #GENRE
    pub genre: Option<String>,
    /// The title of the score. #TITLE
    pub title: Option<String>,
    /// The subtitle of the score. #SUBTITLE
    pub subtitle: Option<String>,
    /// The artist of the music in the score. #ARTIST
    pub artist: Option<String>,
    /// The co-artist of the music in the score. #SUBARTIST
    pub sub_artist: Option<String>,
    /// Who placed the notes into the score. #MAKER
    pub maker: Option<String>,
    /// The text messages of the score. It may be closed with double quotes. #COMMENT
    pub comment: Option<Vec<String>>,
    /// The email address of the author. #EMAIL
    pub email: Option<String>,
    /// The url of the author. #URL
    pub url: Option<String>,
    /// The message for overriding options of some BMS player. #OPTION
    pub options: Option<Vec<String>>,
    /// The initial BPM of the score. #BPM
    pub bpm: Option<GenericFraction<BigUint>>,
    /// The play level of the score. #PLAYLEVEL
    pub play_level: Option<u8>,
    /// The judgement level of the score. #RANK
    pub rank: Option<JudgeLevel>,
    /// The difficulty of the score. #DIFFICULTY
    pub difficulty: Option<u8>,
    /// The total gauge percentage when all notes is got as PERFECT. #TOTAL
    pub total: Option<GenericFraction<BigUint>>,
    /// The volume of the score. #VOLWAV
    pub volume: Volume,
    /// The LN notation type of the score. #LNTYPE
    pub ln_type: LnType,
    /// The display mode for background image/video. #POORBGA
    pub poor_bga_mode: PoorMode,
    /// The path of background image, which is shown while playing the score. #BACKBMP
    pub back_bmp: Option<PathBuf>,
    /// The path of splash screen image, which is shown before playing the score. #STAGEFILE
    pub stage_file: Option<PathBuf>,
    /// The path of banner image. #BANNER
    pub banner: Option<PathBuf>,
    /// Whether the score is the octave mode. #OCT/FP
    pub is_octave: bool,
    /// The path of MIDI file, which is played as BGM while playing the score. #MIDIFILE
    pub midi_file: Option<PathBuf>,
    /// The path of the background video. The video should be started the playing from the section 000. #VIDEOFILE
    pub video_file: Option<PathBuf>,
    /// The path to override the base path of the WAV file path. #PATH_WAV
    pub wav_path_root: Option<PathBuf>,
    /// The WAV file paths corresponding to the id of the note object. #WAV
    pub wav_files: HashMap<ObjId, PathBuf>,
    /// The path of image, which is shown when the player got POOR. #BMP00
    pub poor_bmp: Option<PathBuf>,
    /// The BMP file paths corresponding to the id of the background image/video object. #BMP
    pub bmp_files: HashMap<ObjId, Bmp>,
    /// The BPMs corresponding to the id of the BPM change object. #BPM[01-ZZ]
    pub bpm_changes: HashMap<ObjId, GenericFraction<BigUint>>,
    /// The scrolling factors corresponding to the id of the scroll speed change object. #SCROLL
    pub scrolling_factor_changes: HashMap<ObjId, GenericFraction<BigUint>>,
    /// The spacing factors corresponding to the id of the spacing change object. #SPEED
    pub spacing_factor_changes: HashMap<ObjId, GenericFraction<BigUint>>,
    /// The texts corresponding to the id of the text object. #TEXT
    pub texts: HashMap<ObjId, String>,
    /// The option messages corresponding to the id of the change option object. #CHANGEOPTION
    pub change_options: HashMap<ObjId, String>,
    /// Stop lengths by stop object id. #STOP
    pub stops: HashMap<ObjId, GenericFraction<BigUint>>,
    /// Storage for #@BGA definitions
    pub atbga_defs: HashMap<ObjId, AtBgaDef>,
    /// Storage for #BGA definitions
    pub bga_defs: HashMap<ObjId, BgaDef>,
    /// Storage for #EXRANK definitions
    pub exrank_defs: HashMap<ObjId, ExRankDef>,
    /// Storage for #EXWAV definitions
    pub exwav_defs: HashMap<ObjId, ExWavDef>,
    /// BM98 #ExtChr extended character customization events. #ExtChr
    pub extchr_events: Vec<crate::lex::command::ExtChrEvent>,
    /// The char file path. #CHARFILE
    pub char_file: Option<PathBuf>,
    /// The base BPM. #BASEBPM
    pub base_bpm: Option<GenericFraction<BigUint>>,
    /// Extended ARGB definitions. #ARGB
    pub argb_defs: HashMap<ObjId, Argb>,
    /// Video file frame rate. #VIDEOF/S
    pub video_fs: Option<GenericFraction<BigUint>>,
    /// Video color depth. #VIDEOCOLORS
    pub video_colors: Option<u8>,
    /// Video delay extension. #VIDEODLY
    pub video_dly: Option<GenericFraction<BigUint>>,
    /// Preview audio file for music selection. #PREVIEW
    pub preview: Option<PathBuf>,
    /// Explicitly specify LN type for this chart. #LNMODE
    pub ln_mode: Option<LnModeType>,
    /// Key Bind Layer Animation definitions. #SWBGA
    pub swbga_defs: HashMap<ObjId, SwBgaEvent>,
    /// Global video file. #MOVIE
    pub movie: Option<PathBuf>,
    /// The resolution of Measure of BMS. #DIVIDEPROP
    pub divide_prop: Option<String>,
    /// Charset declaration. #CHARSET
    pub charset: Option<String>,
    /// Extended judge rank definition. #DEFEXRANK
    pub def_exrank: Option<BigUint>,
    /// CD-DA BGM definition. #CDDA
    pub cdda: Option<BigUint>,
    /// Seek events. #SEEK
    pub seek_events: HashMap<ObjId, GenericFraction<BigUint>>,
}

impl Header {
    pub(crate) fn parse(
        &mut self,
        token: &Token,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match token {
            Token::Artist(artist) => self.artist = Some((*artist).into()),
            Token::AtBga {
                id,
                source_bmp,
                trim_top_left,
                trim_size,
                draw_point,
            } => {
                let to_insert = AtBgaDef {
                    id: *id,
                    source_bmp: *source_bmp,
                    trim_top_left: (*trim_top_left).into(),
                    trim_size: (*trim_size).into(),
                    draw_point: (*draw_point).into(),
                };
                if let Some(older) = self.atbga_defs.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::AtBga {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.atbga_defs.insert(*id, to_insert);
                }
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
                let to_insert = BgaDef {
                    id: *id,
                    source_bmp: *source_bmp,
                    trim_top_left: (*trim_top_left).into(),
                    trim_bottom_right: (*trim_bottom_right).into(),
                    draw_point: (*draw_point).into(),
                };
                if let Some(older) = self.bga_defs.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Bga {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.bga_defs.insert(*id, to_insert);
                }
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
                self.bpm = Some(bpm.clone());
            }
            Token::BpmChange(id, bpm) => {
                if let Some(older) = self.bpm_changes.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::BpmChange {
                            id: *id,
                            older: older.to_f64().unwrap(),
                            newer: bpm.to_f64().unwrap(),
                        })
                        .apply(older, bpm.clone())?;
                } else {
                    self.bpm_changes.insert(*id, bpm.clone());
                }
            }
            Token::ChangeOption(id, option) => {
                if let Some(older) = self.change_options.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::ChangeOption {
                            id: *id,
                            older,
                            newer: option,
                        })
                        .apply(older, option.to_string())?;
                } else {
                    self.change_options.insert(*id, option.to_string());
                }
            }
            Token::Comment(comment) => self
                .comment
                .get_or_insert_with(Vec::new)
                .push(comment.to_string()),
            Token::Difficulty(diff) => self.difficulty = Some(*diff),
            Token::Email(email) => self.email = Some(email.to_string()),
            Token::ExBmp(id, transparent_color, path) => {
                let to_insert = Bmp {
                    file: path.into(),
                    transparent_color: *transparent_color,
                };
                if let Some(older) = self.bmp_files.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Bmp {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.bmp_files.insert(*id, to_insert);
                }
            }
            Token::ExRank(id, judge_level) => {
                let to_insert = ExRankDef {
                    id: *id,
                    judge_level: *judge_level,
                };
                if let Some(older) = self.exrank_defs.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::ExRank {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.exrank_defs.insert(*id, to_insert);
                }
            }
            Token::ExWav {
                id,
                pan,
                volume,
                frequency,
                path,
            } => {
                let to_insert = ExWavDef {
                    id: *id,
                    pan: *pan,
                    volume: *volume,
                    frequency: *frequency,
                    path: path.into(),
                };
                if let Some(older) = self.exwav_defs.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::ExWav {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.exwav_defs.insert(*id, to_insert);
                }
            }
            Token::Genre(genre) => self.genre = Some(genre.to_string()),
            Token::LnTypeRdm => {
                self.ln_type = LnType::Rdm;
            }
            Token::LnTypeMgq => {
                self.ln_type = LnType::Mgq;
            }
            Token::Maker(maker) => self.maker = Some(maker.to_string()),
            Token::MidiFile(midi_file) => self.midi_file = Some(midi_file.into()),
            Token::OctFp => self.is_octave = true,
            Token::Option(option) => self
                .options
                .get_or_insert_with(Vec::new)
                .push(option.to_string()),
            Token::PathWav(wav_path_root) => self.wav_path_root = Some(wav_path_root.into()),
            Token::Player(player) => self.player = Some(*player),
            Token::PlayLevel(play_level) => self.play_level = Some(*play_level),
            Token::PoorBga(poor_bga_mode) => self.poor_bga_mode = *poor_bga_mode,
            Token::Rank(rank) => self.rank = Some(*rank),
            Token::Scroll(id, factor) => {
                if let Some(older) = self.scrolling_factor_changes.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::ScrollingFactorChange {
                            id: *id,
                            older: older.to_f64().unwrap(),
                            newer: factor.to_f64().unwrap(),
                        })
                        .apply(older, factor.clone())?;
                } else {
                    self.scrolling_factor_changes.insert(*id, factor.clone());
                }
            }
            Token::Speed(id, factor) => {
                if let Some(older) = self.spacing_factor_changes.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::SpacingFactorChange {
                            id: *id,
                            older: older.to_f64().unwrap(),
                            newer: factor.to_f64().unwrap(),
                        })
                        .apply(older, factor.clone())?;
                } else {
                    self.spacing_factor_changes.insert(*id, factor.clone());
                }
            }
            Token::StageFile(file) => self.stage_file = Some(file.into()),
            Token::Stop(id, len) => {
                self.stops
                    .entry(*id)
                    .and_modify(|current_len| *current_len = current_len.clone() + len.clone())
                    .or_insert(len.clone());
            }
            Token::SubArtist(sub_artist) => self.sub_artist = Some(sub_artist.to_string()),
            Token::SubTitle(subtitle) => self.subtitle = Some(subtitle.to_string()),
            Token::Text(id, text) => {
                if let Some(older) = self.texts.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Text {
                            id: *id,
                            older,
                            newer: text,
                        })
                        .apply(older, text.to_string())?;
                } else {
                    self.texts.insert(*id, text.to_string());
                }
            }
            Token::Title(title) => self.title = Some(title.to_string()),
            Token::Total(total) => {
                self.total = Some(total.clone());
            }
            Token::Url(url) => self.url = Some(url.to_string()),
            Token::VideoFile(video_file) => self.video_file = Some(video_file.into()),
            Token::VolWav(volume) => self.volume = *volume,
            Token::Wav(id, path) => {
                if let Some(older) = self.wav_files.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Wav {
                            id: *id,
                            older,
                            newer: path,
                        })
                        .apply(older, path.into())?;
                } else {
                    self.wav_files.insert(*id, path.into());
                }
            }
            Token::Random(_)
            | Token::SetRandom(_)
            | Token::If(_)
            | Token::ElseIf(_)
            | Token::Else
            | Token::EndIf
            | Token::EndRandom
            | Token::Switch(_)
            | Token::SetSwitch(_)
            | Token::Case(_)
            | Token::Def
            | Token::Skip
            | Token::EndSwitch => {
                unreachable!()
            }
            Token::Base62
            | Token::LnObj(_)
            | Token::ExtendedMessage { .. }
            | Token::Message { .. } => {
                // These Token should not be handled in Header::parse.
            }
            Token::UnknownCommand(_) | Token::NotACommand(_) => {
                // this token should be handled outside.
            }
            Token::CharFile(path) => {
                // Only store the latest value
                self.char_file = Some(path.into());
            }
            Token::BaseBpm(bpm) => {
                // Only store the latest value
                self.base_bpm = Some(bpm.clone());
            }
            Token::Argb(id, argb) => {
                // Only store the latest value, report error if duplicated
                if let Some(older) = self.argb_defs.get_mut(id) {
                    return Err(crate::parse::ParseWarning::SyntaxError(format!(
                        "Duplicated ARGB definition for id {id:?}. value {older:?} is throwed."
                    )));
                } else {
                    self.argb_defs.insert(*id, *argb);
                }
            }
            Token::VideoFs(fs) => {
                self.video_fs = Some(fs.clone());
            }
            Token::VideoColors(rgb) => {
                self.video_colors = Some(*rgb);
            }
            Token::VideoDly(dly) => {
                self.video_dly = Some(dly.clone());
            }
            Token::Preview(path) => {
                self.preview = Some(path.into());
            }
            Token::LnMode(mode) => {
                self.ln_mode = Some(*mode);
            }
            Token::ExtChr(ev) => {
                self.extchr_events.push(*ev);
            }
            Token::MaterialsWav(_) | Token::MaterialsBmp(_) => {
                // These tokens are not stored in Header, just ignore
            }
            Token::DivideProp(s) => {
                self.divide_prop = Some(s.to_string());
            }
            Token::Charset(s) => {
                self.charset = Some(s.to_string());
            }
            Token::DefExRank(val) => {
                self.def_exrank = Some(val.clone());
            }
            Token::SwBga(id, ev) => {
                if let Some(older) = self.swbga_defs.get_mut(id) {
                    return Err(crate::parse::ParseWarning::SyntaxError(format!(
                        "Duplicated SWBGA definition for id {id:?}. value {older:?} is throwed."
                    )));
                } else {
                    self.swbga_defs.insert(*id, ev.clone());
                }
            }
            Token::Movie(path) => {
                self.movie = Some(path.into());
            }
            Token::Cdda(val) => {
                self.cdda = Some(val.clone());
            }
            Token::Stp(_) | Token::WavCmd(_) | Token::Seek(_, _) => {
                // These tokens are not stored in Header, just ignore
            }
        }
        Ok(())
    }
}
