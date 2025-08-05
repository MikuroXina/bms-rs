//! Header information from parsed BMS file.
//! Note objects manager.

pub mod def;
pub mod obj;

use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    ops::Bound,
    path::PathBuf,
    str::FromStr,
};

use fraction::GenericFraction;
use itertools::Itertools;
#[cfg(feature = "minor-command")]
use num::BigUint;

#[cfg(feature = "minor-command")]
use crate::bms::command::{
    ExtChrEvent,
    minor_command::{StpEvent, SwBgaEvent, WavCmdEvent},
};
use crate::bms::{
    Decimal,
    command::{
        Argb, JudgeLevel, LnMode, LnType, ObjId, PlayerMode, PoorMode, Volume,
        channel::{Channel, Key, NoteKind},
        time::{ObjTime, Track},
    },
    lex::token::{Token, TokenContent},
};

#[cfg(feature = "minor-command")]
use self::def::{AtBgaDef, BgaDef, ExWavDef};
use self::{
    def::{Bmp, ExRankDef},
    obj::{
        BgaLayer, BgaObj, BpmChangeObj, ExtendedMessageObj, Obj, ScrollingFactorObj,
        SectionLenChangeObj, SpeedObj, StopObj,
    },
};
use super::{
    ParseWarningContent, Result,
    prompt::{PromptHandler, PromptingDuplication},
};

/// A score data of BMS format.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bms {
    /// The header data in the score.
    pub header: Header,
    /// The scope-defines in the score.
    pub scope_defines: ScopeDefines,
    /// The arranges in the score.
    pub arrangers: Arrangers,
    /// The objects in the score.
    pub notes: Notes,
    /// The graphics part in the score.
    pub graphics: Graphics,
    /// The other part in the score.
    pub others: Others,
}

/// A header of the score, including the information that is usually used in music selection.
/// Parsed from [`TokenStream`](crate::lex::TokenContent::TokenStream).
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
    /// The play level of the score.
    pub play_level: Option<u8>,
    /// The judgement level of the score.
    pub rank: Option<JudgeLevel>,
    /// The difficulty of the score.
    pub difficulty: Option<u8>,
    /// The total gauge percentage when all notes is got as PERFECT.
    pub total: Option<Decimal>,
    /// The volume of the score.
    pub volume: Volume,
    /// The LN notation type of the score.
    pub ln_type: LnType,
    /// The path of background image, which is shown while playing the score.
    pub back_bmp: Option<PathBuf>,
    /// The path of splash screen image, which is shown before playing the score.
    pub stage_file: Option<PathBuf>,
    /// The path of banner image.
    pub banner: Option<PathBuf>,
    /// LN Mode
    pub ln_mode: LnMode,
    /// Preview Music
    pub preview_music: Option<PathBuf>,
    /// Movie Define
    pub movie: Option<PathBuf>,
}

/// Stores the original scope-defines like `#WAVXX`. Using HashMap.
/// Only stores the original scope-defines, not the parsed ones.
/// Only stores which affects playing.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScopeDefines {
    /// BPM change definitions, indexed by ObjId. #BPM[01-ZZ]
    pub bpm_defs: HashMap<ObjId, Decimal>,
    /// Stop definitions, indexed by ObjId. #STOP[01-ZZ]
    pub stop_defs: HashMap<ObjId, Decimal>,
    /// Scroll speed change definitions, indexed by ObjId. #SCROLL[01-ZZ]
    pub scroll_defs: HashMap<ObjId, Decimal>,
    /// Spacing change definitions, indexed by ObjId. #SPEED[01-ZZ]
    pub speed_defs: HashMap<ObjId, Decimal>,
    /// Storage for #EXRANK definitions
    pub exrank_defs: HashMap<ObjId, ExRankDef>,
    /// Storage for #EXWAV definitions
    #[cfg(feature = "minor-command")]
    pub exwav_defs: HashMap<ObjId, ExWavDef>,
    /// WAVCMD events, indexed by wav_index. #WAVCMD
    #[cfg(feature = "minor-command")]
    pub wavcmd_events: HashMap<ObjId, WavCmdEvent>,
    /// Storage for #@BGA definitions
    #[cfg(feature = "minor-command")]
    pub atbga_defs: HashMap<ObjId, AtBgaDef>,
    /// Storage for #BGA definitions
    #[cfg(feature = "minor-command")]
    pub bga_defs: HashMap<ObjId, BgaDef>,
    /// SWBGA events, indexed by ObjId. #SWBGA
    #[cfg(feature = "minor-command")]
    pub swbga_events: HashMap<ObjId, SwBgaEvent>,
    /// ARGB definitions, indexed by ObjId. #ARGB
    #[cfg(feature = "minor-command")]
    pub argb_defs: HashMap<ObjId, Argb>,
}

/// The objects that arrange the playing panel running or showing.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Arrangers {
    /// Section length change events, indexed by track. #SECLEN
    pub section_len_changes: BTreeMap<Track, SectionLenChangeObj>,
    /// The initial BPM of the score.
    pub bpm: Option<Decimal>,
    /// The BPMs corresponding to the id of the BPM change object.
    /// BPM change events, indexed by time. #BPM[01-ZZ] in message
    pub bpm_changes: BTreeMap<ObjTime, BpmChangeObj>,
    /// Stop lengths by stop object id.
    pub stops: BTreeMap<ObjTime, StopObj>,
    /// The scrolling factors corresponding to the id of the scroll speed change object.
    pub scrolling_factor_changes: BTreeMap<ObjTime, ScrollingFactorObj>,
    /// The spacing factors corresponding to the id of the spacing change object.
    pub speed_factor_changes: BTreeMap<ObjTime, SpeedObj>,
    /// bemaniaDX STP events, indexed by ObjTime. #STP
    #[cfg(feature = "minor-command")]
    pub stp_events: BTreeMap<ObjTime, StpEvent>,
    /// #BASEBPM for LR. Replaced by bpm match in LR2.
    #[cfg(feature = "minor-command")]
    pub base_bpm: Option<Decimal>,
}

/// The playable objects set for querying by lane or time.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Notes {
    /// The path to override the base path of the WAV file path.
    pub wav_path_root: Option<PathBuf>,
    /// The WAV file paths corresponding to the id of the note object.
    pub wav_files: HashMap<ObjId, PathBuf>,
    // objects stored in obj is sorted, so it can be searched by bisection method
    /// BGM objects, indexed by time. #XXX01:ZZ... (BGM placement)
    pub bgms: BTreeMap<ObjTime, Vec<ObjId>>,
    /// All note objects, indexed by ObjId. #XXXYY:ZZ... (note placement)
    pub objs: HashMap<ObjId, Vec<Obj>>,
    /// Index for fast key lookup. Used for LN/landmine logic.
    pub ids_by_key: HashMap<Key, BTreeMap<ObjTime, ObjId>>,
    /// Extended message events. #EXT
    pub extended_messages: Vec<ExtendedMessageObj>,
    /// The path of MIDI file, which is played as BGM while playing the score.
    #[cfg(feature = "minor-command")]
    pub midi_file: Option<PathBuf>,
    /// Material WAV file paths. #MATERIALSWAV
    #[cfg(feature = "minor-command")]
    pub materials_wav: Vec<PathBuf>,
}

/// The graphics objects that are used in the score.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Graphics {
    /// The path of the background video. The video should be started the playing from the section 000.
    pub video_file: Option<PathBuf>,
    /// The BMP file paths corresponding to the id of the background image/video object.
    pub bmp_files: HashMap<ObjId, Bmp>,
    /// BGA change events, indexed by time. #BGA, #BGAPOOR, #BGALAYER
    pub bga_changes: BTreeMap<ObjTime, BgaObj>,
    /// The path of image, which is shown when the player got POOR.
    pub poor_bmp: Option<PathBuf>,
    /// The display mode for background image/video.
    pub poor_bga_mode: PoorMode,
    /// Material BMP file paths. #MATERIALSBMP
    #[cfg(feature = "minor-command")]
    pub materials_bmp: Vec<PathBuf>,
    /// Character file path. #CHARFILE
    #[cfg(feature = "minor-command")]
    pub char_file: Option<PathBuf>,
    /// Video color depth. #VIDEOCOLORS
    #[cfg(feature = "minor-command")]
    pub video_colors: Option<u8>,
    /// Video delay. #VIDEODLY
    #[cfg(feature = "minor-command")]
    pub video_dly: Option<Decimal>,
    /// Video frame rate. #VIDEOF/S
    #[cfg(feature = "minor-command")]
    pub video_fs: Option<Decimal>,
}

/// The other objects that are used in the score. May be arranged in play.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Others {
    /// The message for overriding options of some BMS player.
    pub options: Option<Vec<String>>,
    /// Whether the score is the octave mode.
    #[cfg(feature = "minor-command")]
    pub is_octave: bool,
    /// CDDA events, indexed by value. #CDDA
    #[cfg(feature = "minor-command")]
    pub cdda: Vec<BigUint>,
    /// Seek events, indexed by ObjId. #SEEK
    #[cfg(feature = "minor-command")]
    pub seek_events: HashMap<ObjId, Decimal>,
    /// ExtChr events. #ExtChr
    #[cfg(feature = "minor-command")]
    pub extchr_events: Vec<ExtChrEvent>,
    /// Storage for #TEXT definitions
    /// The texts corresponding to the id of the text object.
    pub texts: HashMap<ObjId, String>,
    /// The option messages corresponding to the id of the change option object.
    pub change_options: HashMap<ObjId, String>,
    /// Lines that not starts with `'#'`.
    pub non_command_lines: Vec<String>,
    /// Lines that starts with `'#'`, but not recognized as vaild command.
    pub unknown_command_lines: Vec<String>,
    /// Divide property. #DIVIDEPROP
    #[cfg(feature = "minor-command")]
    pub divide_prop: Option<String>,
    /// Material path definition. #MATERIALS
    #[cfg(feature = "minor-command")]
    pub materials_path: Option<PathBuf>,
}

impl Bms {
    pub(crate) fn parse(
        &mut self,
        token: &Token,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match &token.content {
            TokenContent::Artist(artist) => self.header.artist = Some(artist.to_string()),
            #[cfg(feature = "minor-command")]
            TokenContent::AtBga {
                id,
                source_bmp,
                trim_top_left,
                trim_size,
                draw_point,
            } => {
                let to_insert = AtBgaDef {
                    id: *id,
                    source_bmp: *source_bmp,
                    trim_top_left: trim_top_left.to_owned().into(),
                    trim_size: trim_size.to_owned().into(),
                    draw_point: draw_point.to_owned().into(),
                };
                if let Some(older) = self.scope_defines.atbga_defs.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::AtBga {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.scope_defines.atbga_defs.insert(*id, to_insert);
                }
            }
            TokenContent::Banner(file) => self.header.banner = Some(file.into()),
            TokenContent::BackBmp(bmp) => self.header.back_bmp = Some(bmp.into()),
            #[cfg(feature = "minor-command")]
            TokenContent::Bga {
                id,
                source_bmp,
                trim_top_left,
                trim_bottom_right,
                draw_point,
            } => {
                let to_insert = BgaDef {
                    id: *id,
                    source_bmp: *source_bmp,
                    trim_top_left: trim_top_left.to_owned().into(),
                    trim_bottom_right: trim_bottom_right.to_owned().into(),
                    draw_point: draw_point.to_owned().into(),
                };
                if let Some(older) = self.scope_defines.bga_defs.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Bga {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.scope_defines.bga_defs.insert(*id, to_insert);
                }
            }
            TokenContent::Bmp(id, path) => {
                if id.is_none() {
                    self.graphics.poor_bmp = Some(path.into());
                    return Ok(());
                }
                let id = id.ok_or(ParseWarningContent::SyntaxError(
                    "BMP id should not be None".to_string(),
                ))?;
                let to_insert = Bmp {
                    file: path.into(),
                    transparent_color: Argb::default(),
                };
                if let Some(older) = self.graphics.bmp_files.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Bmp {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.graphics.bmp_files.insert(id, to_insert);
                }
            }
            TokenContent::Bpm(bpm) => {
                self.arrangers.bpm = Some(bpm.clone());
            }
            TokenContent::BpmChange(id, bpm) => {
                if let Some(older) = self.scope_defines.bpm_defs.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::BpmChange {
                            id: *id,
                            older: older.clone(),
                            newer: bpm.clone(),
                        })
                        .apply(older, bpm.clone())?;
                } else {
                    self.scope_defines.bpm_defs.insert(*id, bpm.clone());
                }
            }
            TokenContent::ChangeOption(id, option) => {
                if let Some(older) = self.others.change_options.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::ChangeOption {
                            id: *id,
                            older,
                            newer: option,
                        })
                        .apply(older, option.to_string())?;
                } else {
                    self.others.change_options.insert(*id, option.to_string());
                }
            }
            TokenContent::Comment(comment) => self
                .header
                .comment
                .get_or_insert_with(Vec::new)
                .push(comment.to_string()),
            TokenContent::Difficulty(diff) => self.header.difficulty = Some(*diff),
            TokenContent::Email(email) => self.header.email = Some(email.to_string()),
            TokenContent::ExBmp(id, transparent_color, path) => {
                let to_insert = Bmp {
                    file: path.into(),
                    transparent_color: *transparent_color,
                };
                if let Some(older) = self.graphics.bmp_files.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Bmp {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.graphics.bmp_files.insert(*id, to_insert);
                }
            }
            TokenContent::ExRank(id, judge_level) => {
                let to_insert = ExRankDef {
                    id: *id,
                    judge_level: *judge_level,
                };
                if let Some(older) = self.scope_defines.exrank_defs.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::ExRank {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.scope_defines.exrank_defs.insert(*id, to_insert);
                }
            }
            #[cfg(feature = "minor-command")]
            TokenContent::ExWav {
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
                if let Some(older) = self.scope_defines.exwav_defs.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::ExWav {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.scope_defines.exwav_defs.insert(*id, to_insert);
                }
            }
            TokenContent::Genre(genre) => self.header.genre = Some(genre.to_string()),
            TokenContent::LnTypeRdm => {
                self.header.ln_type = LnType::Rdm;
            }
            TokenContent::LnTypeMgq => {
                self.header.ln_type = LnType::Mgq;
            }
            TokenContent::Maker(maker) => self.header.maker = Some(maker.to_string()),
            #[cfg(feature = "minor-command")]
            TokenContent::MidiFile(midi_file) => self.notes.midi_file = Some(midi_file.into()),
            #[cfg(feature = "minor-command")]
            TokenContent::OctFp => self.others.is_octave = true,
            TokenContent::Option(option) => self
                .others
                .options
                .get_or_insert_with(Vec::new)
                .push(option.to_string()),
            TokenContent::PathWav(wav_path_root) => {
                self.notes.wav_path_root = Some(wav_path_root.into())
            }
            TokenContent::Player(player) => self.header.player = Some(*player),
            TokenContent::PlayLevel(play_level) => self.header.play_level = Some(*play_level),
            TokenContent::PoorBga(poor_bga_mode) => self.graphics.poor_bga_mode = *poor_bga_mode,
            TokenContent::Rank(rank) => self.header.rank = Some(*rank),
            TokenContent::Scroll(id, factor) => {
                if let Some(older) = self.scope_defines.scroll_defs.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::ScrollingFactorChange {
                            id: *id,
                            older: older.clone(),
                            newer: factor.clone(),
                        })
                        .apply(older, factor.clone())?;
                } else {
                    self.scope_defines.scroll_defs.insert(*id, factor.clone());
                }
            }
            TokenContent::Speed(id, factor) => {
                if let Some(older) = self.scope_defines.speed_defs.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::SpeedFactorChange {
                            id: *id,
                            older: older.clone(),
                            newer: factor.clone(),
                        })
                        .apply(older, factor.clone())?;
                } else {
                    self.scope_defines.speed_defs.insert(*id, factor.clone());
                }
            }
            TokenContent::StageFile(file) => self.header.stage_file = Some(file.into()),
            TokenContent::Stop(id, len) => {
                if let Some(older) = self.scope_defines.stop_defs.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Stop {
                            id: *id,
                            older: older.clone(),
                            newer: len.clone(),
                        })
                        .apply(older, len.clone())?;
                } else {
                    self.scope_defines.stop_defs.insert(*id, len.clone());
                }
            }
            TokenContent::SubArtist(sub_artist) => {
                self.header.sub_artist = Some(sub_artist.to_string())
            }
            TokenContent::SubTitle(subtitle) => self.header.subtitle = Some(subtitle.to_string()),
            TokenContent::Text(id, text) => {
                if let Some(older) = self.others.texts.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Text {
                            id: *id,
                            older,
                            newer: text,
                        })
                        .apply(older, text.to_string())?;
                } else {
                    self.others.texts.insert(*id, text.to_string());
                }
            }
            TokenContent::Title(title) => self.header.title = Some(title.to_string()),
            TokenContent::Total(total) => {
                self.header.total = Some(total.clone());
            }
            TokenContent::Url(url) => self.header.url = Some(url.to_string()),
            TokenContent::VideoFile(video_file) => {
                self.graphics.video_file = Some(video_file.into())
            }
            TokenContent::VolWav(volume) => self.header.volume = *volume,
            TokenContent::Wav(id, path) => {
                if let Some(older) = self.notes.wav_files.get_mut(id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Wav {
                            id: *id,
                            older,
                            newer: path,
                        })
                        .apply(older, path.into())?;
                } else {
                    self.notes.wav_files.insert(*id, path.into());
                }
            }
            #[cfg(feature = "minor-command")]
            TokenContent::Stp(ev) => {
                // Store by ObjTime as key, report error if duplicated
                let key = ev.time;
                if self.arrangers.stp_events.contains_key(&key) {
                    return Err(super::ParseWarningContent::SyntaxError(format!(
                        "Duplicated STP event at time {key:?}"
                    )));
                }
                self.arrangers.stp_events.insert(key, *ev);
            }
            #[cfg(feature = "minor-command")]
            TokenContent::WavCmd(ev) => {
                // Store by wav_index as key, report error if duplicated
                let key = ev.wav_index;
                if self.scope_defines.wavcmd_events.contains_key(&key) {
                    return Err(super::ParseWarningContent::SyntaxError(format!(
                        "Duplicated WAVCMD event for wav_index {key:?}",
                    )));
                }
                self.scope_defines.wavcmd_events.insert(key, *ev);
            }
            #[cfg(feature = "minor-command")]
            TokenContent::SwBga(id, ev) => {
                if self.scope_defines.swbga_events.contains_key(id) {
                    return Err(super::ParseWarningContent::SyntaxError(format!(
                        "Duplicated SWBGA event for id {id:?}",
                    )));
                }
                self.scope_defines.swbga_events.insert(*id, ev.clone());
            }
            #[cfg(feature = "minor-command")]
            TokenContent::Argb(id, argb) => {
                if self.scope_defines.argb_defs.contains_key(id) {
                    return Err(super::ParseWarningContent::SyntaxError(format!(
                        "Duplicated ARGB definition for id {id:?}",
                    )));
                }
                self.scope_defines.argb_defs.insert(*id, *argb);
            }
            #[cfg(feature = "minor-command")]
            TokenContent::Seek(id, v) => {
                if self.others.seek_events.contains_key(id) {
                    return Err(super::ParseWarningContent::SyntaxError(format!(
                        "Duplicated Seek event for id {id:?}",
                    )));
                }
                self.others.seek_events.insert(*id, v.clone());
            }
            #[cfg(feature = "minor-command")]
            TokenContent::ExtChr(ev) => {
                self.others.extchr_events.push(*ev);
            }
            #[cfg(feature = "minor-command")]
            TokenContent::MaterialsWav(path) => {
                self.notes.materials_wav.push(path.to_path_buf());
            }
            #[cfg(feature = "minor-command")]
            TokenContent::MaterialsBmp(path) => {
                self.graphics.materials_bmp.push(path.to_path_buf());
            }
            TokenContent::Message {
                track,
                channel: Channel::BpmChange,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let bpm = self
                        .scope_defines
                        .bpm_defs
                        .get(&obj)
                        .ok_or(ParseWarningContent::UndefinedObject(obj))?;
                    self.arrangers.push_bpm_change(
                        BpmChangeObj {
                            time,
                            bpm: bpm.clone(),
                        },
                        prompt_handler,
                    )?;
                }
            }
            TokenContent::Message {
                track,
                channel: Channel::BpmChangeU8,
                message,
            } => {
                let denominator = message.len() as u64 / 2;
                for (i, (c1, c2)) in message.chars().tuples().enumerate() {
                    let bpm = c1
                        .to_digit(16)
                        .ok_or(ParseWarningContent::SyntaxError(format!(
                            "Invalid hex digit: {c1}",
                        )))?
                        * 16
                        + c2.to_digit(16)
                            .ok_or(ParseWarningContent::SyntaxError(format!(
                                "Invalid hex digit: {c2}",
                            )))?;
                    if bpm == 0 {
                        continue;
                    }
                    let time = ObjTime::new(track.0, i as u64, denominator);
                    self.arrangers.push_bpm_change(
                        BpmChangeObj {
                            time,
                            bpm: Decimal::from(bpm),
                        },
                        prompt_handler,
                    )?;
                }
            }
            TokenContent::Message {
                track,
                channel: Channel::Scroll,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let factor = self
                        .scope_defines
                        .scroll_defs
                        .get(&obj)
                        .ok_or(ParseWarningContent::UndefinedObject(obj))?;
                    self.arrangers.push_scrolling_factor_change(
                        ScrollingFactorObj {
                            time,
                            factor: factor.clone(),
                        },
                        prompt_handler,
                    )?;
                }
            }
            TokenContent::Message {
                track,
                channel: Channel::Speed,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let factor = self
                        .scope_defines
                        .speed_defs
                        .get(&obj)
                        .ok_or(ParseWarningContent::UndefinedObject(obj))?;
                    self.arrangers.push_speed_factor_change(
                        SpeedObj {
                            time,
                            factor: factor.clone(),
                        },
                        prompt_handler,
                    )?;
                }
            }
            TokenContent::Message {
                track,
                channel: Channel::ChangeOption,
                message,
            } => {
                for (_time, obj) in ids_from_message(*track, message) {
                    let _option = self
                        .others
                        .change_options
                        .get(&obj)
                        .ok_or(ParseWarningContent::UndefinedObject(obj))?;
                    // Here we can add logic to handle ChangeOption
                    // Currently just ignored because change_options are already stored in notes
                }
            }
            TokenContent::Message {
                track,
                channel: Channel::SectionLen,
                message,
            } => {
                let length = Decimal::from(Decimal::from_fraction(
                    GenericFraction::from_str(message).map_err(|_| {
                        ParseWarningContent::SyntaxError(format!(
                            "Invalid section length: {message}"
                        ))
                    })?,
                ));
                if length <= Decimal::from(0u64) {
                    return Err(ParseWarningContent::SyntaxError(
                        "section length must be greater than zero".to_string(),
                    ));
                }
                self.arrangers.push_section_len_change(
                    SectionLenChangeObj {
                        track: *track,
                        length,
                    },
                    prompt_handler,
                )?;
            }
            TokenContent::Message {
                track,
                channel: Channel::Stop,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let duration = self
                        .scope_defines
                        .stop_defs
                        .get(&obj)
                        .ok_or(ParseWarningContent::UndefinedObject(obj))?;
                    self.arrangers.push_stop(StopObj {
                        time,
                        duration: duration.clone(),
                    })
                }
            }
            TokenContent::Message {
                track,
                channel: channel @ (Channel::BgaBase | Channel::BgaPoor | Channel::BgaLayer),
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    if !self.graphics.bmp_files.contains_key(&obj) {
                        return Err(ParseWarningContent::UndefinedObject(obj));
                    }
                    let layer = match channel {
                        Channel::BgaBase => BgaLayer::Base,
                        Channel::BgaPoor => BgaLayer::Poor,
                        Channel::BgaLayer => BgaLayer::Overlay,
                        _ => unreachable!(),
                    };
                    self.graphics.push_bga_change(
                        BgaObj {
                            time,
                            id: obj,
                            layer,
                        },
                        prompt_handler,
                    )?;
                }
            }
            TokenContent::Message {
                track,
                channel: Channel::Bgm,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    self.notes.bgms.entry(time).or_default().push(obj)
                }
            }
            TokenContent::Message {
                track,
                channel: Channel::Note { kind, side, key },
                message,
            } => {
                for (offset, obj) in ids_from_message(*track, message) {
                    self.notes.push_note(Obj {
                        offset,
                        kind: *kind,
                        side: *side,
                        key: *key,
                        obj,
                    });
                }
            }
            TokenContent::ExtendedMessage {
                track,
                channel,
                message,
            } => {
                self.notes.push_extended_message(ExtendedMessageObj {
                    track: *track,
                    channel: channel.clone(),
                    message: (*message).to_owned(),
                });
            }
            TokenContent::LnObj(end_id) => {
                let mut end_note = self
                    .notes
                    .remove_latest_note(*end_id)
                    .ok_or(ParseWarningContent::UndefinedObject(*end_id))?;
                let Obj { offset, key, .. } = &end_note;
                let (_, &begin_id) = self.notes.ids_by_key[key]
                    .range(..offset)
                    .last()
                    .ok_or_else(|| {
                        ParseWarningContent::SyntaxError(format!(
                            "expected preceding object for #LNOBJ {end_id:?}",
                        ))
                    })?;
                let mut begin_note = self.notes.remove_latest_note(begin_id).ok_or(
                    ParseWarningContent::SyntaxError(format!(
                        "Cannot find begin note for LNOBJ {end_id:?}"
                    )),
                )?;
                begin_note.kind = NoteKind::Long;
                end_note.kind = NoteKind::Long;
                self.notes.push_note(begin_note);
                self.notes.push_note(end_note);
            }
            TokenContent::DefExRank(judge_level) => {
                let judge_level = JudgeLevel::OtherInt(*judge_level as i64);
                self.scope_defines.exrank_defs.insert(
                    ObjId::try_from([0, 0]).map_err(|_| {
                        ParseWarningContent::SyntaxError("Invalid ObjId [0, 0]".to_string())
                    })?,
                    ExRankDef {
                        id: ObjId::try_from([0, 0]).map_err(|_| {
                            ParseWarningContent::SyntaxError("Invalid ObjId [0, 0]".to_string())
                        })?,
                        judge_level,
                    },
                );
            }
            TokenContent::LnMode(ln_mode_type) => {
                self.header.ln_mode = *ln_mode_type;
            }
            TokenContent::Movie(path) => self.header.movie = Some(path.into()),
            TokenContent::Preview(path) => self.header.preview_music = Some(path.into()),
            #[cfg(feature = "minor-command")]
            TokenContent::Cdda(big_uint) => self.others.cdda.push(big_uint.clone()),
            #[cfg(feature = "minor-command")]
            TokenContent::BaseBpm(generic_decimal) => {
                self.arrangers.base_bpm = Some(generic_decimal.clone())
            }
            TokenContent::NotACommand(line) => self.others.non_command_lines.push(line.to_string()),
            TokenContent::UnknownCommand(line) => {
                self.others.unknown_command_lines.push(line.to_string())
            }
            TokenContent::Base62 | TokenContent::Charset(_) => {
                // Pass.
            }
            TokenContent::Random(_)
            | TokenContent::SetRandom(_)
            | TokenContent::If(_)
            | TokenContent::ElseIf(_)
            | TokenContent::Else
            | TokenContent::EndIf
            | TokenContent::EndRandom
            | TokenContent::Switch(_)
            | TokenContent::SetSwitch(_)
            | TokenContent::Case(_)
            | TokenContent::Def
            | TokenContent::Skip
            | TokenContent::EndSwitch => {
                unreachable!()
            }
            #[cfg(feature = "minor-command")]
            TokenContent::CharFile(path) => {
                self.graphics.char_file = Some(path.into());
            }
            #[cfg(feature = "minor-command")]
            TokenContent::DivideProp(prop) => {
                self.others.divide_prop = Some(prop.to_string());
            }
            #[cfg(feature = "minor-command")]
            TokenContent::Materials(path) => {
                self.others.materials_path = Some(path.to_path_buf());
            }
            #[cfg(feature = "minor-command")]
            TokenContent::VideoColors(colors) => {
                self.graphics.video_colors = Some(*colors);
            }
            #[cfg(feature = "minor-command")]
            TokenContent::VideoDly(delay) => {
                self.graphics.video_dly = Some(delay.clone());
            }
            #[cfg(feature = "minor-command")]
            TokenContent::VideoFs(frame_rate) => {
                self.graphics.video_fs = Some(frame_rate.clone());
            }
        }
        Ok(())
    }
}

impl Bms {
    /// Gets the time of last any object including visible, BGM, BPM change, section length change and so on.
    ///
    /// You can't use this to find the length of music. Because this doesn't consider that the length of sound.
    pub fn last_obj_time(&self) -> Option<ObjTime> {
        let obj_last = self
            .notes
            .objs
            .values()
            .flatten()
            .map(Reverse)
            .sorted()
            .next()
            .map(|Reverse(obj)| obj.offset);
        let bpm_last = self
            .arrangers
            .bpm_changes
            .last_key_value()
            .map(|(&time, _)| time);
        let section_len_last =
            self.arrangers
                .section_len_changes
                .last_key_value()
                .map(|(&time, _)| ObjTime {
                    track: time,
                    numerator: 0,
                    denominator: 4,
                });
        let stop_last = self.arrangers.stops.last_key_value().map(|(&time, _)| time);
        let bga_last = self
            .graphics
            .bga_changes
            .last_key_value()
            .map(|(&time, _)| time);
        [obj_last, bpm_last, section_len_last, stop_last, bga_last]
            .into_iter()
            .max()
            .flatten()
    }

    /// Calculates a required resolution to convert the notes time into pulses, which split one quarter note evenly.
    pub fn resolution_for_pulses(&self) -> u64 {
        use num::Integer;

        let mut hyp_resolution = 1u64;
        for obj in self.notes.objs.values().flatten() {
            hyp_resolution = hyp_resolution.lcm(&obj.offset.denominator);
        }
        for bpm_change in self.arrangers.bpm_changes.values() {
            hyp_resolution = hyp_resolution.lcm(&bpm_change.time.denominator);
        }
        hyp_resolution
    }
}

impl Arrangers {
    /// Adds a new BPM change object to the notes.
    pub fn push_bpm_change(
        &mut self,
        bpm_change: BpmChangeObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.bpm_changes.entry(bpm_change.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(bpm_change);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_duplication(PromptingDuplication::BpmChangeEvent {
                        time: bpm_change.time,
                        older: existing,
                        newer: &bpm_change,
                    })
                    .apply(entry.get_mut(), bpm_change)
            }
        }
    }

    /// Adds a new scrolling factor change object to the notes.
    pub fn push_scrolling_factor_change(
        &mut self,
        scrolling_factor_change: ScrollingFactorObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self
            .scrolling_factor_changes
            .entry(scrolling_factor_change.time)
        {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(scrolling_factor_change);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_duplication(PromptingDuplication::ScrollingFactorChangeEvent {
                        time: scrolling_factor_change.time,
                        older: existing,
                        newer: &scrolling_factor_change,
                    })
                    .apply(entry.get_mut(), scrolling_factor_change)
            }
        }
    }

    /// Adds a new spacing factor change object to the notes.
    pub fn push_speed_factor_change(
        &mut self,
        speed_factor_change: SpeedObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.speed_factor_changes.entry(speed_factor_change.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(speed_factor_change);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_duplication(PromptingDuplication::SpeedFactorChangeEvent {
                        time: speed_factor_change.time,
                        older: existing,
                        newer: &speed_factor_change,
                    })
                    .apply(entry.get_mut(), speed_factor_change)
            }
        }
    }

    /// Adds a new section length change object to the notes.
    pub fn push_section_len_change(
        &mut self,
        section_len_change: SectionLenChangeObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.section_len_changes.entry(section_len_change.track) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(section_len_change);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_duplication(PromptingDuplication::SectionLenChangeEvent {
                        track: section_len_change.track,
                        older: existing,
                        newer: &section_len_change,
                    })
                    .apply(entry.get_mut(), section_len_change)
            }
        }
    }

    /// Adds a new stop object to the notes.
    pub fn push_stop(&mut self, stop: StopObj) {
        self.stops
            .entry(stop.time)
            .and_modify(|existing| {
                existing.duration = &existing.duration + &stop.duration;
            })
            .or_insert(stop.clone());
    }
}

impl Graphics {
    /// Returns the bga change objects.
    pub fn bga_changes(&self) -> &BTreeMap<ObjTime, BgaObj> {
        &self.bga_changes
    }

    /// Adds a new bga change object to the notes.
    pub fn push_bga_change(
        &mut self,
        bga: BgaObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.bga_changes.entry(bga.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(bga);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_duplication(PromptingDuplication::BgaChangeEvent {
                        time: bga.time,
                        older: existing,
                        newer: &bga,
                    })
                    .apply(entry.get_mut(), bga)
            }
        }
    }
}

impl Notes {
    /// Converts into the notes sorted by time.
    pub fn into_all_notes(self) -> Vec<Obj> {
        self.objs.into_values().flatten().sorted().collect()
    }

    /// Returns the iterator having all of the notes sorted by time.
    pub fn all_notes(&self) -> impl Iterator<Item = &Obj> {
        self.objs.values().flatten().sorted()
    }

    /// Returns all the bgms in the score.
    pub fn bgms(&self) -> &BTreeMap<ObjTime, Vec<ObjId>> {
        &self.bgms
    }

    /// Finds next object on the key `Key` from the time `ObjTime`.
    pub fn next_obj_by_key(&self, key: Key, time: ObjTime) -> Option<&Obj> {
        self.ids_by_key
            .get(&key)?
            .range((Bound::Excluded(time), Bound::Unbounded))
            .next()
            .and_then(|(_, id)| {
                let objs = self.objs.get(id)?;
                let idx = objs
                    .binary_search_by(|probe| probe.offset.cmp(&time))
                    .unwrap_or_else(|idx| idx);
                objs.get(idx)
            })
    }

    /// Adds the new note object to the notes.
    pub fn push_note(&mut self, note: Obj) {
        self.objs.entry(note.obj).or_default().push(note.clone());
        self.ids_by_key
            .entry(note.key)
            .or_default()
            .insert(note.offset, note.obj);
    }

    /// Removes the latest note from the notes.
    pub fn remove_latest_note(&mut self, id: ObjId) -> Option<Obj> {
        self.objs.entry(id).or_default().pop().inspect(|removed| {
            if let Some(key_map) = self.ids_by_key.get_mut(&removed.key) {
                key_map.remove(&removed.offset);
            }
        })
    }

    /// Removes the note from the notes.
    pub fn remove_note(&mut self, id: ObjId) -> Vec<Obj> {
        self.objs.remove(&id).map_or(vec![], |removed| {
            for item in &removed {
                if let Some(key_map) = self.ids_by_key.get_mut(&item.key) {
                    key_map.remove(&item.offset);
                }
            }
            removed
        })
    }

    /// Adds the new extended message object to the notes.
    pub fn push_extended_message(&mut self, message: ExtendedMessageObj) {
        self.extended_messages.push(message);
    }

    /// Gets the time of last visible object.
    pub fn last_visible_time(&self) -> Option<ObjTime> {
        self.objs
            .values()
            .flatten()
            .filter(|obj| !matches!(obj.kind, NoteKind::Invisible))
            .map(Reverse)
            .sorted()
            .next()
            .map(|Reverse(obj)| obj.offset)
    }

    /// Gets the time of last BGM object.
    ///
    /// You can't use this to find the length of music. Because this doesn't consider that the length of sound. And visible notes may ring after all BGMs.
    pub fn last_bgm_time(&self) -> Option<ObjTime> {
        self.bgms.last_key_value().map(|(time, _)| time).cloned()
    }
}

fn ids_from_message(track: Track, message: &'_ str) -> impl Iterator<Item = (ObjTime, ObjId)> + '_ {
    let denominator = message.len() as u64 / 2;
    let mut chars = message.chars().tuples().enumerate();
    std::iter::from_fn(move || {
        let (i, c1, c2) = loop {
            let (i, (c1, c2)) = chars.next()?;
            if !(c1 == '0' && c2 == '0') {
                break (i, c1, c2);
            }
        };
        let obj = ObjId::try_from([c1, c2]).ok()?;
        let time = ObjTime::new(track.0, i as u64, denominator);
        Some((time, obj))
    })
}
