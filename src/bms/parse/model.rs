//! Header information from parsed BMS file.
//! Note objects manager.

pub mod def;
pub mod notes_pack;
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
use crate::bms::command::{ExtChrEvent, StpEvent, SwBgaEvent, WavCmdEvent};
use crate::bms::{
    Decimal,
    command::{
        Argb, Channel, JudgeLevel, Key, LnType, NoteKind, ObjId, ObjTime, PlayerMode, PoorMode,
        Track, Volume,
    },
    lex::token::Token,
};

#[cfg(feature = "minor-command")]
use self::def::{AtBgaDef, BgaDef, ExWavDef};
use self::{
    def::{Bmp, ExRankDef},
    obj::{
        BgaLayer, BgaObj, BpmChangeObj, ExtendedMessageObj, Obj, ScrollingFactorObj,
        SectionLenChangeObj, SpacingFactorObj, StopObj,
    },
};
use super::{
    ParseWarning, Result,
    prompt::{PromptHandler, PromptingDuplication},
};

/// A score data of BMS format.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bms {
    /// The header data in the score.
    pub header: Header,
    /// The objects in the score.
    pub notes: Notes,
    /// Lines that not starts with `'#'`.
    pub non_command_lines: Vec<String>,
    /// Lines that starts with `'#'`, but not recognized as vaild command.
    pub unknown_command_lines: Vec<String>,
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
    pub bpm: Option<Decimal>,
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
    /// The display mode for background image/video.
    pub poor_bga_mode: PoorMode,
    /// The path of background image, which is shown while playing the score.
    pub back_bmp: Option<PathBuf>,
    /// The path of splash screen image, which is shown before playing the score.
    pub stage_file: Option<PathBuf>,
    /// The path of banner image.
    pub banner: Option<PathBuf>,
    /// Whether the score is the octave mode.
    #[cfg(feature = "minor-command")]
    pub is_octave: bool,
    /// The path of MIDI file, which is played as BGM while playing the score.
    #[cfg(feature = "minor-command")]
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
    pub bpm_changes: HashMap<ObjId, Decimal>,
    /// The scrolling factors corresponding to the id of the scroll speed change object.
    pub scrolling_factor_changes: HashMap<ObjId, Decimal>,
    /// The spacing factors corresponding to the id of the spacing change object.
    pub spacing_factor_changes: HashMap<ObjId, Decimal>,
    /// The texts corresponding to the id of the text object.
    pub texts: HashMap<ObjId, String>,
    /// The option messages corresponding to the id of the change option object.
    pub change_options: HashMap<ObjId, String>,
    /// Stop lengths by stop object id.
    pub stops: HashMap<ObjId, Decimal>,
    /// Storage for #@BGA definitions
    #[cfg(feature = "minor-command")]
    pub atbga_defs: HashMap<ObjId, AtBgaDef>,
    /// Storage for #BGA definitions
    #[cfg(feature = "minor-command")]
    pub bga_defs: HashMap<ObjId, BgaDef>,
    /// Storage for #EXRANK definitions
    pub exrank_defs: HashMap<ObjId, ExRankDef>,
    /// Storage for #EXWAV definitions
    #[cfg(feature = "minor-command")]
    pub exwav_defs: HashMap<ObjId, ExWavDef>,
    /// bemaniaDX STP events, indexed by ObjTime. #STP
    #[cfg(feature = "minor-command")]
    pub stp_events: HashMap<ObjTime, StpEvent>,
    /// WAVCMD events, indexed by wav_index. #WAVCMD
    #[cfg(feature = "minor-command")]
    pub wavcmd_events: HashMap<ObjId, WavCmdEvent>,
    /// CDDA events, indexed by value. #CDDA
    #[cfg(feature = "minor-command")]
    pub cdda_events: HashMap<u64, u64>,
    /// SWBGA events, indexed by ObjId. #SWBGA
    #[cfg(feature = "minor-command")]
    pub swbga_events: HashMap<ObjId, SwBgaEvent>,
    /// ARGB definitions, indexed by ObjId. #ARGB
    #[cfg(feature = "minor-command")]
    pub argb_defs: HashMap<ObjId, Argb>,
    /// Seek events, indexed by ObjId. #SEEK
    #[cfg(feature = "minor-command")]
    pub seek_events: HashMap<ObjId, Decimal>,
    /// ExtChr events. #ExtChr
    #[cfg(feature = "minor-command")]
    pub extchr_events: Vec<ExtChrEvent>,
    /// Material WAV file paths. #MATERIALSWAV
    #[cfg(feature = "minor-command")]
    pub materials_wav: Vec<PathBuf>,
    /// Material BMP file paths. #MATERIALSBMP
    #[cfg(feature = "minor-command")]
    pub materials_bmp: Vec<PathBuf>,
}

/// The objects set for querying by lane or time.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Notes {
    // objects stored in obj is sorted, so it can be searched by bisection method
    /// All note objects, indexed by ObjId. #XXXYY:ZZ... (note placement)
    pub objs: HashMap<ObjId, Vec<Obj>>,
    /// BGM objects, indexed by time. #XXX01:ZZ... (BGM placement)
    pub bgms: BTreeMap<ObjTime, Vec<ObjId>>,
    /// Index for fast key lookup. Used for LN/landmine logic.
    pub ids_by_key: HashMap<Key, BTreeMap<ObjTime, ObjId>>,
    /// BPM change events, indexed by time. #BPM[01-ZZ] in message
    pub bpm_changes: BTreeMap<ObjTime, BpmChangeObj>,
    /// Section length change events, indexed by track. #SECLEN
    pub section_len_changes: BTreeMap<Track, SectionLenChangeObj>,
    /// Stop events, indexed by time. #STOP[01-ZZ] in message
    pub stops: BTreeMap<ObjTime, StopObj>,
    /// BGA change events, indexed by time. #BGA, #BGAPOOR, #BGALAYER
    pub bga_changes: BTreeMap<ObjTime, BgaObj>,
    /// Scrolling factor change events, indexed by time. #SCROLL in message
    pub scrolling_factor_changes: BTreeMap<ObjTime, ScrollingFactorObj>,
    /// Spacing factor change events, indexed by time. #SPEED in message
    pub spacing_factor_changes: BTreeMap<ObjTime, SpacingFactorObj>,
    /// Extended message events. #EXT
    pub extended_messages: Vec<ExtendedMessageObj>,
    /// Storage for #EXRANK definitions
    pub exrank_defs: HashMap<ObjId, ExRankDef>,
    /// Storage for #EXWAV definitions
    #[cfg(feature = "minor-command")]
    pub exwav_defs: HashMap<ObjId, ExWavDef>,
    /// Storage for #CHANGEOPTION definitions
    pub change_options: HashMap<ObjId, String>,
    /// Storage for #TEXT definitions
    pub texts: HashMap<ObjId, String>,
    /// bemaniaDX STP events, indexed by ObjTime. #STP
    #[cfg(feature = "minor-command")]
    pub stp_events: HashMap<ObjTime, StpEvent>,
    /// WAVCMD events, indexed by wav_index. #WAVCMD
    #[cfg(feature = "minor-command")]
    pub wavcmd_events: HashMap<ObjId, WavCmdEvent>,
    /// CDDA events, indexed by value. #CDDA
    #[cfg(feature = "minor-command")]
    pub cdda_events: HashMap<u64, u64>,
    /// SWBGA events, indexed by ObjId. #SWBGA
    #[cfg(feature = "minor-command")]
    pub swbga_events: HashMap<ObjId, SwBgaEvent>,
    /// ARGB definitions, indexed by ObjId. #ARGB
    #[cfg(feature = "minor-command")]
    pub argb_defs: HashMap<ObjId, Argb>,
    /// Seek events, indexed by ObjId. #SEEK
    #[cfg(feature = "minor-command")]
    pub seek_events: HashMap<ObjId, Decimal>,
}

impl Header {
    pub(crate) fn parse(
        &mut self,
        token: &Token,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match *token {
            Token::Artist(artist) => self.artist = Some(artist.into()),
            #[cfg(feature = "minor-command")]
            Token::AtBga {
                id,
                source_bmp,
                trim_top_left,
                trim_size,
                draw_point,
            } => {
                let to_insert = AtBgaDef {
                    id,
                    source_bmp,
                    trim_top_left: trim_top_left.into(),
                    trim_size: trim_size.into(),
                    draw_point: draw_point.into(),
                };
                if let Some(older) = self.atbga_defs.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::AtBga {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.atbga_defs.insert(id, to_insert);
                }
            }
            Token::Banner(file) => self.banner = Some(file.into()),
            Token::BackBmp(bmp) => self.back_bmp = Some(bmp.into()),
            #[cfg(feature = "minor-command")]
            Token::Bga {
                id,
                source_bmp,
                trim_top_left,
                trim_bottom_right,
                draw_point,
            } => {
                let to_insert = BgaDef {
                    id,
                    source_bmp,
                    trim_top_left: trim_top_left.into(),
                    trim_bottom_right: trim_bottom_right.into(),
                    draw_point: draw_point.into(),
                };
                if let Some(older) = self.bga_defs.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Bga {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.bga_defs.insert(id, to_insert);
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
            Token::Bpm(ref bpm) => {
                self.bpm = Some(bpm.clone());
            }
            Token::BpmChange(id, ref bpm) => {
                if let Some(older) = self.bpm_changes.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::BpmChange {
                            id,
                            older: older.clone(),
                            newer: bpm.clone(),
                        })
                        .apply(older, bpm.clone())?;
                } else {
                    self.bpm_changes.insert(id, bpm.clone());
                }
            }
            Token::ChangeOption(id, option) => {
                if let Some(older) = self.change_options.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::ChangeOption {
                            id,
                            older,
                            newer: option,
                        })
                        .apply(older, option.into())?;
                } else {
                    self.change_options.insert(id, option.into());
                }
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
                let to_insert = ExRankDef { id, judge_level };
                if let Some(older) = self.exrank_defs.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::ExRank {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.exrank_defs.insert(id, to_insert);
                }
            }
            #[cfg(feature = "minor-command")]
            Token::ExWav {
                id,
                pan,
                volume,
                frequency,
                path,
            } => {
                let to_insert = ExWavDef {
                    id,
                    pan,
                    volume,
                    frequency,
                    path: path.into(),
                };
                if let Some(older) = self.exwav_defs.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::ExWav {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply(older, to_insert)?;
                } else {
                    self.exwav_defs.insert(id, to_insert);
                }
            }
            Token::Genre(genre) => self.genre = Some(genre.to_owned()),
            Token::LnTypeRdm => {
                self.ln_type = LnType::Rdm;
            }
            Token::LnTypeMgq => {
                self.ln_type = LnType::Mgq;
            }
            Token::Maker(maker) => self.maker = Some(maker.into()),
            #[cfg(feature = "minor-command")]
            Token::MidiFile(midi_file) => self.midi_file = Some(midi_file.into()),
            #[cfg(feature = "minor-command")]
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
            Token::Scroll(id, ref factor) => {
                if let Some(older) = self.scrolling_factor_changes.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::ScrollingFactorChange {
                            id,
                            older: older.clone(),
                            newer: factor.clone(),
                        })
                        .apply(older, factor.clone())?;
                } else {
                    self.scrolling_factor_changes.insert(id, factor.clone());
                }
            }
            Token::Speed(id, ref factor) => {
                if let Some(older) = self.spacing_factor_changes.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::SpacingFactorChange {
                            id,
                            older: older.clone(),
                            newer: factor.clone(),
                        })
                        .apply(older, factor.clone())?;
                } else {
                    self.spacing_factor_changes.insert(id, factor.clone());
                }
            }
            Token::StageFile(file) => self.stage_file = Some(file.into()),
            Token::Stop(id, ref len) => {
                self.stops
                    .entry(id)
                    .and_modify(|current_len| *current_len += len.clone())
                    .or_insert(len.clone());
            }
            Token::SubArtist(sub_artist) => self.sub_artist = Some(sub_artist.into()),
            Token::SubTitle(subtitle) => self.subtitle = Some(subtitle.into()),
            Token::Text(id, text) => {
                if let Some(older) = self.texts.get_mut(&id) {
                    prompt_handler
                        .handle_duplication(PromptingDuplication::Text {
                            id,
                            older,
                            newer: text,
                        })
                        .apply(older, text.into())?;
                } else {
                    self.texts.insert(id, text.into());
                }
            }
            Token::Title(title) => self.title = Some(title.into()),
            Token::Total(ref total) => {
                self.total = Some(total.clone());
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
            #[cfg(feature = "minor-command")]
            Token::Stp(ev) => {
                // Store by ObjTime as key, report error if duplicated
                let key = ev.time;
                if self.stp_events.contains_key(&key) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated STP event at time {key:?}"
                    )));
                }
                self.stp_events.insert(key, ev);
            }
            #[cfg(feature = "minor-command")]
            Token::WavCmd(ev) => {
                // Store by wav_index as key, report error if duplicated
                let key = ev.wav_index;
                if self.wavcmd_events.contains_key(&key) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated WAVCMD event for wav_index {key:?}",
                    )));
                }
                self.wavcmd_events.insert(key, ev);
            }
            #[cfg(feature = "minor-command")]
            Token::SwBga(id, ref ev) => {
                if self.swbga_events.contains_key(&id) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated SWBGA event for id {id:?}",
                    )));
                }
                self.swbga_events.insert(id, ev.clone());
            }
            #[cfg(feature = "minor-command")]
            Token::Argb(id, argb) => {
                if self.argb_defs.contains_key(&id) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated ARGB definition for id {id:?}",
                    )));
                }
                self.argb_defs.insert(id, argb);
            }
            #[cfg(feature = "minor-command")]
            Token::Seek(id, ref v) => {
                if self.seek_events.contains_key(&id) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated Seek event for id {id:?}",
                    )));
                }
                self.seek_events.insert(id, v.clone());
            }
            #[cfg(feature = "minor-command")]
            Token::ExtChr(ev) => {
                self.extchr_events.push(ev);
            }
            #[cfg(feature = "minor-command")]
            Token::MaterialsWav(path) => {
                self.materials_wav.push(path.into());
            }
            #[cfg(feature = "minor-command")]
            Token::MaterialsBmp(path) => {
                self.materials_bmp.push(path.into());
            }
            Token::Movie(_) | Token::LnMode(_) | Token::Preview(_) | Token::Charset(_) => {
                // These tokens are not stored in Notes, just ignore
            }
            // Control flow
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
            | Token::DefExRank(_)
            | Token::Message { .. } => {
                // These Token should not be handled in Header::parse.
            }
            #[cfg(feature = "minor-command")]
            Token::CharFile(_)
            | Token::BaseBpm(_)
            | Token::DivideProp(_)
            | Token::VideoFs(_)
            | Token::VideoColors(_)
            | Token::VideoDly(_)
            | Token::Cdda(_) => {
                // These tokens are not stored in Notes, just ignore
            }
            Token::UnknownCommand(_) | Token::NotACommand(_) => {
                // this token should be handled outside.
            }
        }
        Ok(())
    }
}

impl Notes {
    /// Creates a new notes dictionary.
    pub fn new() -> Self {
        Default::default()
    }

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

    /// Returns the bpm change objects.
    pub fn bpm_changes(&self) -> &BTreeMap<ObjTime, BpmChangeObj> {
        &self.bpm_changes
    }

    /// Returns the scrolling factor change objects.
    pub fn scrolling_factor_changes(&self) -> &BTreeMap<ObjTime, ScrollingFactorObj> {
        &self.scrolling_factor_changes
    }

    /// Returns the spacing factor change objects.
    pub fn spacing_factor_changes(&self) -> &BTreeMap<ObjTime, SpacingFactorObj> {
        &self.spacing_factor_changes
    }

    /// Returns the section len change objects.
    pub fn section_len_changes(&self) -> &BTreeMap<Track, SectionLenChangeObj> {
        &self.section_len_changes
    }

    /// Returns the scroll stop objects.
    pub fn stops(&self) -> &BTreeMap<ObjTime, StopObj> {
        &self.stops
    }

    /// Returns the bga change objects.
    pub fn bga_changes(&self) -> &BTreeMap<ObjTime, BgaObj> {
        &self.bga_changes
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
            self.ids_by_key
                .get_mut(&removed.key)
                .unwrap()
                .remove(&removed.offset)
                .unwrap();
        })
    }

    /// Removes the note from the notes.
    pub fn remove_note(&mut self, id: ObjId) -> Vec<Obj> {
        self.objs.remove(&id).map_or(vec![], |removed| {
            for item in &removed {
                self.ids_by_key
                    .get_mut(&item.key)
                    .unwrap()
                    .remove(&item.offset)
                    .unwrap();
            }
            removed
        })
    }

    /// Adds a new BPM change object to the notes.
    pub fn push_bpm_change(&mut self, bpm_change: BpmChangeObj) {
        if let Some(existing) = self.bpm_changes.insert(bpm_change.time, bpm_change) {
            eprintln!(
                "duplicate bpm change object detected at {:?}",
                existing.time
            );
        }
    }

    /// Adds a new scrolling factor change object to the notes.
    pub fn push_scrolling_factor_change(&mut self, bpm_change: ScrollingFactorObj) {
        if let Some(existing) = self
            .scrolling_factor_changes
            .insert(bpm_change.time, bpm_change.clone())
        {
            eprintln!(
                "duplicate scrolling factor change object detected at {:?}",
                existing.time
            );
        }
    }

    /// Adds a new spacing factor change object to the notes.
    pub fn push_spacing_factor_change(&mut self, bpm_change: SpacingFactorObj) {
        if let Some(existing) = self
            .spacing_factor_changes
            .insert(bpm_change.time, bpm_change.clone())
        {
            eprintln!(
                "duplicate spacing factor change object detected at {:?}",
                existing.time
            );
        }
    }

    /// Adds a new section length change object to the notes.
    pub fn push_section_len_change(&mut self, section_len_change: SectionLenChangeObj) {
        if let Some(existing) = self
            .section_len_changes
            .insert(section_len_change.track, section_len_change.clone())
        {
            eprintln!(
                "duplicate section length change object detected at {:?}",
                existing.track
            );
        }
    }

    /// Adds a new stop object to the notes.
    pub fn push_stop(&mut self, stop: StopObj) {
        self.stops
            .entry(stop.time)
            .and_modify(|existing| {
                existing.duration = existing.duration.clone() + stop.duration.clone();
            })
            .or_insert(stop.clone());
    }

    /// Adds a new bga change object to the notes.
    pub fn push_bga_change(&mut self, bga: BgaObj) {
        if let Some(existing) = self.bga_changes.insert(bga.time, bga) {
            eprintln!(
                "duplicate bga change object detected at {:?}",
                existing.time
            );
        }
    }

    /// Adds the new extended message object to the notes.
    pub fn push_extended_message(&mut self, message: ExtendedMessageObj) {
        self.extended_messages.push(message);
    }

    pub(crate) fn parse(&mut self, token: &Token, header: &Header) -> Result<()> {
        match token {
            Token::Message {
                track,
                channel: Channel::BpmChange,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let bpm = header
                        .bpm_changes
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    self.push_bpm_change(BpmChangeObj {
                        time,
                        bpm: bpm.clone(),
                    });
                }
            }
            Token::Message {
                track,
                channel: Channel::BpmChangeU8,
                message,
            } => {
                let denominator = message.len() as u64 / 2;
                for (i, (c1, c2)) in message.chars().tuples().enumerate() {
                    let bpm = c1.to_digit(16).unwrap() * 16 + c2.to_digit(16).unwrap();
                    if bpm == 0 {
                        continue;
                    }
                    let time = ObjTime::new(track.0, i as u64, denominator);
                    self.push_bpm_change(BpmChangeObj {
                        time,
                        bpm: Decimal::from(bpm),
                    });
                }
            }
            Token::Message {
                track,
                channel: Channel::Scroll,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let factor = header
                        .scrolling_factor_changes
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    self.push_scrolling_factor_change(ScrollingFactorObj {
                        time,
                        factor: factor.clone(),
                    });
                }
            }
            Token::Message {
                track,
                channel: Channel::Speed,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let factor = header
                        .spacing_factor_changes
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    self.push_spacing_factor_change(SpacingFactorObj {
                        time,
                        factor: factor.clone(),
                    });
                }
            }
            Token::Message {
                track,
                channel: Channel::ChangeOption,
                message,
            } => {
                for (_time, obj) in ids_from_message(*track, message) {
                    let _option = header
                        .change_options
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    // Here we can add logic to handle ChangeOption
                    // Currently just ignored because change_options are already stored in header
                }
            }
            Token::Message {
                track,
                channel: Channel::SectionLen,
                message,
            } => {
                let track = Track(track.0);
                let length = Decimal::from(Decimal::from_fraction(
                    GenericFraction::from_str(message).expect("f64 as section length"),
                ));
                assert!(
                    length > Decimal::from(0u64),
                    "section length must be greater than zero"
                );
                self.push_section_len_change(SectionLenChangeObj { track, length });
            }
            Token::Message {
                track,
                channel: Channel::Stop,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let duration = header
                        .stops
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    self.push_stop(StopObj {
                        time,
                        duration: duration.clone(),
                    })
                }
            }
            Token::Message {
                track,
                channel: channel @ (Channel::BgaBase | Channel::BgaPoor | Channel::BgaLayer),
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    if !header.bmp_files.contains_key(&obj) {
                        return Err(ParseWarning::UndefinedObject(obj));
                    }
                    let layer = match channel {
                        Channel::BgaBase => BgaLayer::Base,
                        Channel::BgaPoor => BgaLayer::Poor,
                        Channel::BgaLayer => BgaLayer::Overlay,
                        _ => unreachable!(),
                    };
                    self.push_bga_change(BgaObj {
                        time,
                        id: obj,
                        layer,
                    });
                }
            }
            Token::Message {
                track,
                channel: Channel::Bgm,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    self.bgms.entry(time).or_default().push(obj)
                }
            }
            Token::Message {
                track,
                channel: Channel::Note { kind, side, key },
                message,
            } => {
                for (offset, obj) in ids_from_message(*track, message) {
                    self.push_note(Obj {
                        offset,
                        kind: *kind,
                        side: *side,
                        key: *key,
                        obj,
                    });
                }
            }
            Token::ExtendedMessage {
                track,
                channel,
                message,
            } => {
                let track = Track(track.0);
                self.push_extended_message(ExtendedMessageObj {
                    track,
                    channel: channel.clone(),
                    message: (*message).to_owned(),
                });
            }
            &Token::LnObj(end_id) => {
                let mut end_note = self
                    .remove_latest_note(end_id)
                    .ok_or(ParseWarning::UndefinedObject(end_id))?;
                let Obj { offset, key, .. } = &end_note;
                let (_, &begin_id) =
                    self.ids_by_key[key].range(..offset).last().ok_or_else(|| {
                        ParseWarning::SyntaxError(format!(
                            "expected preceding object for #LNOBJ {end_id:?}",
                        ))
                    })?;
                let mut begin_note = self.remove_latest_note(begin_id).unwrap();
                begin_note.kind = NoteKind::Long;
                end_note.kind = NoteKind::Long;
                self.push_note(begin_note);
                self.push_note(end_note);
            }
            Token::ExRank(id, judge_level) => {
                self.exrank_defs.insert(
                    *id,
                    ExRankDef {
                        id: *id,
                        judge_level: *judge_level,
                    },
                );
            }
            #[cfg(feature = "minor-command")]
            Token::ExWav {
                id,
                pan,
                volume,
                frequency,
                path,
            } => {
                self.exwav_defs.insert(
                    *id,
                    ExWavDef {
                        id: *id,
                        pan: *pan,
                        volume: *volume,
                        frequency: *frequency,
                        path: path.into(),
                    },
                );
            }
            Token::ChangeOption(id, option) => {
                self.change_options.insert(*id, (*option).to_string());
            }
            Token::Text(id, text) => {
                self.texts.insert(*id, (*text).to_string());
            }
            #[cfg(feature = "minor-command")]
            Token::Stp(ev) => {
                // Store by ObjTime as key, report error if duplicated
                let key = ev.time;
                if self.stp_events.contains_key(&key) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated STP event at time {key:?}"
                    )));
                }
                self.stp_events.insert(key, *ev);
            }
            #[cfg(feature = "minor-command")]
            Token::WavCmd(ev) => {
                // Store by wav_index as key, report error if duplicated
                let key = ev.wav_index;
                if self.wavcmd_events.contains_key(&key) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated WAVCMD event for wav_index {key:?}",
                    )));
                }
                self.wavcmd_events.insert(key, *ev);
            }
            #[cfg(feature = "minor-command")]
            Token::SwBga(id, ev) => {
                if self.swbga_events.contains_key(id) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated SWBGA event for id {id:?}",
                    )));
                }
                self.swbga_events.insert(*id, ev.clone());
            }
            #[cfg(feature = "minor-command")]
            Token::Argb(id, argb) => {
                if self.argb_defs.contains_key(id) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated ARGB definition for id {id:?}",
                    )));
                }
                self.argb_defs.insert(*id, *argb);
            }
            #[cfg(feature = "minor-command")]
            Token::Seek(id, v) => {
                if self.seek_events.contains_key(id) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated Seek event for id {id:?}",
                    )));
                }
                self.seek_events.insert(*id, v.clone());
            }
            // Control flow
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
            Token::Email(_)
            | Token::Url(_)
            | Token::Option(_)
            | Token::PathWav(_)
            | Token::Maker(_)
            | Token::PoorBga(_)
            | Token::VideoFile(_)
            | Token::Artist(_)
            | Token::Banner(_)
            | Token::BackBmp(_)
            | Token::Base62
            | Token::Bmp(_, _)
            | Token::Bpm(_)
            | Token::BpmChange(_, _)
            | Token::Comment(_)
            | Token::Difficulty(_)
            | Token::ExBmp(_, _, _)
            | Token::Genre(_)
            | Token::LnTypeRdm
            | Token::LnTypeMgq
            | Token::Player(_)
            | Token::PlayLevel(_)
            | Token::Rank(_)
            | Token::Scroll(_, _)
            | Token::Speed(_, _)
            | Token::StageFile(_)
            | Token::Stop(_, _)
            | Token::SubArtist(_)
            | Token::SubTitle(_)
            | Token::Title(_)
            | Token::Total(_)
            | Token::VolWav(_)
            | Token::Wav(_, _) => {
                // These tokens don't need to be processed in Notes::parse, they should be handled in Header::parse
            }
            Token::Charset(_)
            | Token::DefExRank(_)
            | Token::Preview(_)
            | Token::LnMode(_)
            | Token::Movie(_) => {
                // These tokens are not stored in Notes, just ignore
            }
            #[cfg(feature = "minor-command")]
            Token::CharFile(_)
            | Token::BaseBpm(_)
            | Token::AtBga { .. }
            | Token::Bga { .. }
            | Token::OctFp
            | Token::MidiFile(_)
            | Token::ExtChr(_)
            | Token::MaterialsWav(_)
            | Token::MaterialsBmp(_)
            | Token::DivideProp(_)
            | Token::Cdda(_)
            | Token::VideoFs(_)
            | Token::VideoColors(_)
            | Token::VideoDly(_) => {
                // These tokens are not stored in Notes, just ignore
            }
            Token::UnknownCommand(_) | Token::NotACommand(_) => {
                // this token should be handled outside.
            }
        }
        Ok(())
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

    /// Gets the time of last any object including visible, BGM, BPM change, section length change and so on.
    ///
    /// You can't use this to find the length of music. Because this doesn't consider that the length of sound.
    pub fn last_obj_time(&self) -> Option<ObjTime> {
        let obj_last = self
            .objs
            .values()
            .flatten()
            .map(Reverse)
            .sorted()
            .next()
            .map(|Reverse(obj)| obj.offset);
        let bpm_last = self.bpm_changes.last_key_value().map(|(&time, _)| time);
        let section_len_last =
            self.section_len_changes
                .last_key_value()
                .map(|(&time, _)| ObjTime {
                    track: time,
                    numerator: 0,
                    denominator: 4,
                });
        let stop_last = self.stops.last_key_value().map(|(&time, _)| time);
        let bga_last = self.bga_changes.last_key_value().map(|(&time, _)| time);
        [obj_last, bpm_last, section_len_last, stop_last, bga_last]
            .into_iter()
            .max()
            .flatten()
    }

    /// Calculates a required resolution to convert the notes time into pulses, which split one quarter note evenly.
    pub fn resolution_for_pulses(&self) -> u64 {
        use num::Integer;

        let mut hyp_resolution = 1u64;
        for obj in self.objs.values().flatten() {
            hyp_resolution = hyp_resolution.lcm(&obj.offset.denominator);
        }
        for bpm_change in self.bpm_changes.values() {
            hyp_resolution = hyp_resolution.lcm(&bpm_change.time.denominator);
        }
        hyp_resolution
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
        let obj = ObjId::try_from([c1, c2]).expect("invalid object id");
        let time = ObjTime::new(track.0, i as u64, denominator);
        Some((time, obj))
    })
}
