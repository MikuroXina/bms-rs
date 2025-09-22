//! Parsing Bms from [`TokenStream`].
//!
//! Raw [String] == [lex] ==> [`TokenStream`] (in [`BmsLexOutput`]) == [parse] ==> [Bms] (in
//! [`BmsParseOutput`])

pub mod check_playing;
pub mod prompt;
pub mod validity;

use fraction::GenericFraction;
use itertools::Itertools;
use std::{num::NonZeroU64, str::FromStr};
use thiserror::Error;

use crate::bms::diagnostics::{SimpleSource, ToAriadne};
use ariadne::{Color, Label, Report, ReportKind};

use super::prelude::*;
use crate::bms::{
    ast::{
        AstBuildOutput, AstBuildWarningWithRange, AstParseOutput, AstParseWarningWithRange,
        AstRoot, rng::Rng,
    },
    command::{
        ObjId,
        channel::{
            Channel,
            mapper::{KeyLayoutBeat, KeyLayoutMapper},
        },
        mixin::SourceRangeMixin,
        time::{ObjTime, Track},
    },
    lex::token::{Token, TokenWithRange},
    model::Bms,
};

#[cfg(feature = "minor-command")]
use self::prompt::ChannelDuplication;
use self::prompt::PromptHandler;

/// An error occurred when parsing the [`TokenStream`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ParseWarning {
    /// Syntax formed from the commands was invalid.
    #[error("syntax error: {0}")]
    SyntaxError(String),
    /// The object has required but not defined,
    #[error("undefined object: {0:?}")]
    UndefinedObject(ObjId),
    /// Has duplicated definition, that `prompt_handler` returned [`DuplicationWorkaround::Warn`].
    #[error("duplicating definition: {0}")]
    DuplicatingDef(ObjId),
    /// Has duplicated track object, that `prompt_handler` returned [`DuplicationWorkaround::Warn`].
    #[error("duplicating track object: {0} {1}")]
    DuplicatingTrackObj(Track, Channel),
    /// Has duplicated channel object, that `prompt_handler` returned [`DuplicationWorkaround::Warn`].
    #[error("duplicating channel object: {0} {1}")]
    DuplicatingChannelObj(ObjTime, Channel),
    /// Unexpected control flow.
    #[error("unexpected control flow")]
    UnexpectedControlFlow,
}

/// Type alias of `core::result::Result<T, ParseWarning>`
pub(crate) type Result<T> = core::result::Result<T, ParseWarning>;

/// A parse warning with position information.
pub type ParseWarningWithRange = SourceRangeMixin<ParseWarning>;

/// Bms Parse Output
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use]
pub struct ParseOutput<T: KeyLayoutMapper = KeyLayoutBeat> {
    /// The output Bms.
    pub bms: Bms<T>,
    /// Warnings that occurred during parsing.
    pub parse_warnings: Vec<ParseWarningWithRange>,
}

impl<T: KeyLayoutMapper> Bms<T> {
    /// Parses a token stream into [`Bms`] without AST.
    pub fn from_token_stream<'a>(
        token_iter: impl IntoIterator<Item = &'a TokenWithRange<'a>>,
        mut prompt_handler: impl PromptHandler,
    ) -> ParseOutput<T> {
        let mut bms = Self::default();
        let mut parse_warnings = vec![];
        for token in token_iter {
            if let Err(error) = bms.parse(token, &mut prompt_handler) {
                parse_warnings.push(error.into_wrapper(token));
            }
        }

        ParseOutput {
            bms,
            parse_warnings,
        }
    }
}

impl<T: KeyLayoutMapper> Bms<T> {
    pub(crate) fn parse(
        &mut self,
        token: &TokenWithRange,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match token.content() {
            Token::Artist(artist) => self.header.artist = Some(artist.to_string()),
            #[cfg(feature = "minor-command")]
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
                    trim_top_left: trim_top_left.to_owned().into(),
                    trim_size: trim_size.to_owned().into(),
                    draw_point: draw_point.to_owned().into(),
                };
                if let Some(older) = self.scope_defines.atbga_defs.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::AtBga {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, *id)?;
                } else {
                    self.scope_defines.atbga_defs.insert(*id, to_insert);
                }
            }
            Token::Banner(file) => self.header.banner = Some(file.into()),
            Token::BackBmp(bmp) => self.header.back_bmp = Some(bmp.into()),
            #[cfg(feature = "minor-command")]
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
                    trim_top_left: trim_top_left.to_owned().into(),
                    trim_bottom_right: trim_bottom_right.to_owned().into(),
                    draw_point: draw_point.to_owned().into(),
                };
                if let Some(older) = self.scope_defines.bga_defs.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::Bga {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, *id)?;
                } else {
                    self.scope_defines.bga_defs.insert(*id, to_insert);
                }
            }
            Token::Bmp(id, path) => {
                if id.is_none() {
                    self.graphics.poor_bmp = Some(path.into());
                    return Ok(());
                }
                let id = id.ok_or(ParseWarning::SyntaxError(
                    "BMP id should not be None".to_string(),
                ))?;
                let to_insert = Bmp {
                    file: path.into(),
                    transparent_color: Argb::default(),
                };
                if let Some(older) = self.graphics.bmp_files.get_mut(&id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::Bmp {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, id)?;
                } else {
                    self.graphics.bmp_files.insert(id, to_insert);
                }
            }
            Token::Bpm(bpm) => {
                self.arrangers.bpm = Some(bpm.clone());
            }
            Token::BpmChange(id, bpm) => {
                if let Some(older) = self.scope_defines.bpm_defs.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::BpmChange {
                            id: *id,
                            older: older.clone(),
                            newer: bpm.clone(),
                        })
                        .apply_def(older, bpm.clone(), *id)?;
                } else {
                    self.scope_defines.bpm_defs.insert(*id, bpm.clone());
                }
            }
            #[cfg(feature = "minor-command")]
            Token::ChangeOption(id, option) => {
                if let Some(older) = self.others.change_options.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::ChangeOption {
                            id: *id,
                            older,
                            newer: option,
                        })
                        .apply_def(older, option.to_string(), *id)?;
                } else {
                    self.others.change_options.insert(*id, option.to_string());
                }
            }
            Token::Comment(comment) => self
                .header
                .comment
                .get_or_insert_with(Vec::new)
                .push(comment.to_string()),
            Token::Difficulty(diff) => self.header.difficulty = Some(*diff),
            Token::Email(email) => self.header.email = Some(email.to_string()),
            Token::ExBmp(id, transparent_color, path) => {
                let to_insert = Bmp {
                    file: path.into(),
                    transparent_color: *transparent_color,
                };
                if let Some(older) = self.graphics.bmp_files.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::Bmp {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, *id)?;
                } else {
                    self.graphics.bmp_files.insert(*id, to_insert);
                }
            }
            Token::ExRank(id, judge_level) => {
                let to_insert = ExRankDef {
                    id: *id,
                    judge_level: *judge_level,
                };
                if let Some(older) = self.scope_defines.exrank_defs.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::ExRank {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, *id)?;
                } else {
                    self.scope_defines.exrank_defs.insert(*id, to_insert);
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
                    id: *id,
                    pan: *pan,
                    volume: *volume,
                    frequency: *frequency,
                    path: path.into(),
                };
                if let Some(older) = self.scope_defines.exwav_defs.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::ExWav {
                            id: *id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, *id)?;
                } else {
                    self.scope_defines.exwav_defs.insert(*id, to_insert);
                }
            }
            Token::Genre(genre) => self.header.genre = Some(genre.to_string()),
            Token::LnTypeRdm => {
                self.header.ln_type = LnType::Rdm;
            }
            Token::LnTypeMgq => {
                self.header.ln_type = LnType::Mgq;
            }
            Token::Maker(maker) => self.header.maker = Some(maker.to_string()),
            #[cfg(feature = "minor-command")]
            Token::MidiFile(midi_file) => self.notes.midi_file = Some(midi_file.into()),
            #[cfg(feature = "minor-command")]
            Token::OctFp => self.others.is_octave = true,
            #[cfg(feature = "minor-command")]
            Token::Option(option) => self
                .others
                .options
                .get_or_insert_with(Vec::new)
                .push(option.to_string()),
            Token::PathWav(wav_path_root) => self.notes.wav_path_root = Some(wav_path_root.into()),
            Token::Player(player) => self.header.player = Some(*player),
            Token::PlayLevel(play_level) => self.header.play_level = Some(*play_level),
            Token::PoorBga(poor_bga_mode) => self.graphics.poor_bga_mode = *poor_bga_mode,
            Token::Rank(rank) => self.header.rank = Some(*rank),
            Token::Scroll(id, factor) => {
                if let Some(older) = self.scope_defines.scroll_defs.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::ScrollingFactorChange {
                            id: *id,
                            older: older.clone(),
                            newer: factor.clone(),
                        })
                        .apply_def(older, factor.clone(), *id)?;
                } else {
                    self.scope_defines.scroll_defs.insert(*id, factor.clone());
                }
            }
            Token::Speed(id, factor) => {
                if let Some(older) = self.scope_defines.speed_defs.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::SpeedFactorChange {
                            id: *id,
                            older: older.clone(),
                            newer: factor.clone(),
                        })
                        .apply_def(older, factor.clone(), *id)?;
                } else {
                    self.scope_defines.speed_defs.insert(*id, factor.clone());
                }
            }
            Token::StageFile(file) => self.header.stage_file = Some(file.into()),
            Token::Stop(id, len) => {
                if let Some(older) = self.scope_defines.stop_defs.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::Stop {
                            id: *id,
                            older: older.clone(),
                            newer: len.clone(),
                        })
                        .apply_def(older, len.clone(), *id)?;
                } else {
                    self.scope_defines.stop_defs.insert(*id, len.clone());
                }
            }
            Token::SubArtist(sub_artist) => self.header.sub_artist = Some(sub_artist.to_string()),
            Token::SubTitle(subtitle) => self.header.subtitle = Some(subtitle.to_string()),
            Token::Text(id, text) => {
                if let Some(older) = self.others.texts.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::Text {
                            id: *id,
                            older,
                            newer: text,
                        })
                        .apply_def(older, text.to_string(), *id)?;
                } else {
                    self.others.texts.insert(*id, text.to_string());
                }
            }
            Token::Title(title) => self.header.title = Some(title.to_string()),
            Token::Total(total) => {
                self.header.total = Some(total.clone());
            }
            Token::Url(url) => self.header.url = Some(url.to_string()),
            Token::VideoFile(video_file) => self.graphics.video_file = Some(video_file.into()),
            Token::VolWav(volume) => self.header.volume = *volume,
            Token::Wav(id, path) => {
                if let Some(older) = self.notes.wav_files.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::Wav {
                            id: *id,
                            older,
                            newer: path,
                        })
                        .apply_def(older, path.into(), *id)?;
                } else {
                    self.notes.wav_files.insert(*id, path.into());
                }
            }
            #[cfg(feature = "minor-command")]
            Token::Stp(ev) => {
                // Store by ObjTime as key, handle duplication with prompt handler
                let key = ev.time;
                if let Some(older) = self.arrangers.stp_events.get_mut(&key) {
                    prompt_handler
                        .handle_channel_duplication(ChannelDuplication::StpEvent {
                            time: key,
                            older,
                            newer: ev,
                        })
                        .apply_channel(older, *ev, key, Channel::Stop)?;
                } else {
                    self.arrangers.stp_events.insert(key, *ev);
                }
            }
            #[cfg(feature = "minor-command")]
            Token::WavCmd(ev) => {
                // Store by wav_index as key, handle duplication with prompt handler
                let key = ev.wav_index;
                if let Some(older) = self.scope_defines.wavcmd_events.get_mut(&key) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::WavCmdEvent {
                            wav_index: key,
                            older,
                            newer: ev,
                        })
                        .apply_def(older, *ev, key)?;
                } else {
                    self.scope_defines.wavcmd_events.insert(key, *ev);
                }
            }
            #[cfg(feature = "minor-command")]
            Token::SwBga(id, ev) => {
                if let Some(older) = self.scope_defines.swbga_events.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::SwBgaEvent {
                            id: *id,
                            older,
                            newer: ev,
                        })
                        .apply_def(older, ev.clone(), *id)?;
                } else {
                    self.scope_defines.swbga_events.insert(*id, ev.clone());
                }
            }
            #[cfg(feature = "minor-command")]
            Token::Argb(id, argb) => {
                if let Some(older) = self.scope_defines.argb_defs.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::BgaArgb {
                            id: *id,
                            older,
                            newer: argb,
                        })
                        .apply_def(older, *argb, *id)?;
                } else {
                    self.scope_defines.argb_defs.insert(*id, *argb);
                }
            }
            #[cfg(feature = "minor-command")]
            Token::Seek(id, v) => {
                if let Some(older) = self.others.seek_events.get_mut(id) {
                    prompt_handler
                        .handle_def_duplication(DefDuplication::SeekEvent {
                            id: *id,
                            older,
                            newer: v,
                        })
                        .apply_def(older, v.clone(), *id)?;
                } else {
                    self.others.seek_events.insert(*id, v.clone());
                }
            }
            #[cfg(feature = "minor-command")]
            Token::ExtChr(ev) => {
                self.others.extchr_events.push(*ev);
            }
            #[cfg(feature = "minor-command")]
            Token::MaterialsWav(path) => {
                self.notes.materials_wav.push(path.to_path_buf());
            }
            #[cfg(feature = "minor-command")]
            Token::MaterialsBmp(path) => {
                self.graphics.materials_bmp.push(path.to_path_buf());
            }
            Token::Message {
                track,
                channel: Channel::BpmChange,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    // Record used BPM change id for validity checks
                    self.arrangers.bpm_change_ids_used.insert(obj);
                    let bpm = self
                        .scope_defines
                        .bpm_defs
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    self.arrangers.push_bpm_change(
                        BpmChangeObj {
                            time,
                            bpm: bpm.clone(),
                        },
                        prompt_handler,
                    )?;
                }
            }
            Token::Message {
                track,
                channel: Channel::BpmChangeU8,
                message,
            } => {
                let denominator = NonZeroU64::new(message.len() as u64 / 2).ok_or_else(|| {
                    ParseWarning::SyntaxError("denominator cannot be zero".to_string())
                })?;
                for (i, (c1, c2)) in message.chars().tuples().enumerate() {
                    let bpm = c1.to_digit(16).ok_or_else(|| {
                        ParseWarning::SyntaxError(format!("Invalid hex digit: {c1}",))
                    })? * 16
                        + c2.to_digit(16).ok_or_else(|| {
                            ParseWarning::SyntaxError(format!("Invalid hex digit: {c2}",))
                        })?;
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
            Token::Message {
                track,
                channel: Channel::Scroll,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let factor = self
                        .scope_defines
                        .scroll_defs
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    self.arrangers.push_scrolling_factor_change(
                        ScrollingFactorObj {
                            time,
                            factor: factor.clone(),
                        },
                        prompt_handler,
                    )?;
                }
            }
            Token::Message {
                track,
                channel: Channel::Speed,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let factor = self
                        .scope_defines
                        .speed_defs
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    self.arrangers.push_speed_factor_change(
                        SpeedObj {
                            time,
                            factor: factor.clone(),
                        },
                        prompt_handler,
                    )?;
                }
            }
            #[cfg(feature = "minor-command")]
            Token::Message {
                track,
                channel: Channel::ChangeOption,
                message,
            } => {
                for (_time, obj) in ids_from_message(*track, message) {
                    let _option = self
                        .others
                        .change_options
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    // Here we can add logic to handle ChangeOption
                    // Currently just ignored because change_options are already stored in notes
                }
            }
            Token::Message {
                track,
                channel: Channel::SectionLen,
                message,
            } => {
                let length = Decimal::from(Decimal::from_fraction(
                    GenericFraction::from_str(message).map_err(|_| {
                        ParseWarning::SyntaxError(format!("Invalid section length: {message}"))
                    })?,
                ));
                if length <= Decimal::from(0u64) {
                    return Err(ParseWarning::SyntaxError(
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
            Token::Message {
                track,
                channel: Channel::Stop,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    // Record used STOP id for validity checks
                    self.arrangers.stop_ids_used.insert(obj);
                    let duration = self
                        .scope_defines
                        .stop_defs
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    self.arrangers.push_stop(StopObj {
                        time,
                        duration: duration.clone(),
                    });
                }
            }
            Token::Message {
                track,
                channel:
                    channel @ (Channel::BgaBase
                    | Channel::BgaPoor
                    | Channel::BgaLayer
                    | Channel::BgaLayer2),
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    if !self.graphics.bmp_files.contains_key(&obj) {
                        return Err(ParseWarning::UndefinedObject(obj));
                    }
                    let layer = BgaLayer::from_channel(*channel)
                        .unwrap_or_else(|| panic!("Invalid channel for BgaLayer: {channel:?}"));
                    self.graphics.push_bga_change(
                        BgaObj {
                            time,
                            id: obj,
                            layer,
                        },
                        *channel,
                        prompt_handler,
                    )?;
                }
            }
            Token::Message {
                track,
                channel: Channel::Bgm,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    self.notes.push_bgm(time, obj);
                }
            }
            Token::Message {
                track,
                channel: Channel::Note { channel_id },
                message,
            } => {
                // Parse the channel ID to get note components
                for (offset, obj) in ids_from_message(*track, message) {
                    self.notes.push_note(WavObj {
                        offset,
                        channel_id: *channel_id,
                        wav_id: obj,
                    });
                }
            }
            #[cfg(feature = "minor-command")]
            Token::Message {
                track,
                channel:
                    channel @ (Channel::BgaBaseOpacity
                    | Channel::BgaLayerOpacity
                    | Channel::BgaLayer2Opacity
                    | Channel::BgaPoorOpacity),
                message,
            } => {
                for (time, opacity_value) in opacity_from_message(*track, message) {
                    let layer = BgaLayer::from_channel(*channel)
                        .unwrap_or_else(|| panic!("Invalid channel for BgaLayer: {channel:?}"));
                    self.graphics.push_bga_opacity_change(
                        BgaOpacityObj {
                            time,
                            layer,
                            opacity: opacity_value,
                        },
                        *channel,
                        prompt_handler,
                    )?;
                }
            }
            Token::Message {
                track,
                channel: Channel::BgmVolume,
                message,
            } => {
                for (time, volume_value) in volume_from_message(*track, message) {
                    self.notes.push_bgm_volume_change(
                        BgmVolumeObj {
                            time,
                            volume: volume_value,
                        },
                        prompt_handler,
                    )?;
                }
            }
            Token::Message {
                track,
                channel: Channel::KeyVolume,
                message,
            } => {
                for (time, volume_value) in volume_from_message(*track, message) {
                    self.notes.push_key_volume_change(
                        KeyVolumeObj {
                            time,
                            volume: volume_value,
                        },
                        prompt_handler,
                    )?;
                }
            }
            #[cfg(feature = "minor-command")]
            Token::Message {
                track,
                channel:
                    channel @ (Channel::BgaBaseArgb
                    | Channel::BgaLayerArgb
                    | Channel::BgaLayer2Argb
                    | Channel::BgaPoorArgb),
                message,
            } => {
                for (time, argb_id) in ids_from_message(*track, message) {
                    let layer = BgaLayer::from_channel(*channel)
                        .unwrap_or_else(|| panic!("Invalid channel for BgaLayer: {channel:?}"));
                    let argb = self
                        .scope_defines
                        .argb_defs
                        .get(&argb_id)
                        .ok_or(ParseWarning::UndefinedObject(argb_id))?;
                    self.graphics.push_bga_argb_change(
                        BgaArgbObj {
                            time,
                            layer,
                            argb: *argb,
                        },
                        *channel,
                        prompt_handler,
                    )?;
                }
            }
            #[cfg(feature = "minor-command")]
            Token::Message {
                track,
                channel: Channel::Seek,
                message,
            } => {
                for (time, seek_id) in ids_from_message(*track, message) {
                    let position = self
                        .others
                        .seek_events
                        .get(&seek_id)
                        .ok_or(ParseWarning::UndefinedObject(seek_id))?;
                    self.notes.push_seek_event(
                        SeekObj {
                            time,
                            position: position.clone(),
                        },
                        prompt_handler,
                    )?;
                }
            }
            Token::Message {
                track,
                channel: Channel::Text,
                message,
            } => {
                for (time, text_id) in ids_from_message(*track, message) {
                    let text = self
                        .others
                        .texts
                        .get(&text_id)
                        .ok_or(ParseWarning::UndefinedObject(text_id))?;
                    self.notes.push_text_event(
                        TextObj {
                            time,
                            text: text.clone(),
                        },
                        prompt_handler,
                    )?;
                }
            }
            Token::Message {
                track,
                channel: Channel::Judge,
                message,
            } => {
                for (time, judge_id) in ids_from_message(*track, message) {
                    let exrank_def = self
                        .scope_defines
                        .exrank_defs
                        .get(&judge_id)
                        .ok_or(ParseWarning::UndefinedObject(judge_id))?;
                    self.notes.push_judge_event(
                        JudgeObj {
                            time,
                            judge_level: exrank_def.judge_level,
                        },
                        prompt_handler,
                    )?;
                }
            }
            #[cfg(feature = "minor-command")]
            Token::Message {
                track,
                channel: Channel::BgaKeybound,
                message,
            } => {
                for (time, keybound_id) in ids_from_message(*track, message) {
                    let event = self
                        .scope_defines
                        .swbga_events
                        .get(&keybound_id)
                        .ok_or(ParseWarning::UndefinedObject(keybound_id))?;
                    self.notes.push_bga_keybound_event(
                        BgaKeyboundObj {
                            time,
                            event: event.clone(),
                        },
                        prompt_handler,
                    )?;
                }
            }
            #[cfg(feature = "minor-command")]
            Token::Message {
                track,
                channel: Channel::Option,
                message,
            } => {
                for (time, option_id) in ids_from_message(*track, message) {
                    let option = self
                        .others
                        .change_options
                        .get(&option_id)
                        .ok_or(ParseWarning::UndefinedObject(option_id))?;
                    self.notes.push_option_event(
                        OptionObj {
                            time,
                            option: option.clone(),
                        },
                        prompt_handler,
                    )?;
                }
            }
            Token::LnObj(end_id) => {
                let mut end_note = self
                    .notes
                    .pop_latest_of(*end_id)
                    .ok_or(ParseWarning::UndefinedObject(*end_id))?;
                let WavObj {
                    offset, channel_id, ..
                } = &end_note;
                let begin_idx = self
                    .notes
                    .notes_in(..offset)
                    .rev()
                    .find(|(_, obj)| obj.channel_id == *channel_id)
                    .ok_or_else(|| {
                        ParseWarning::SyntaxError(format!(
                            "expected preceding object for #LNOBJ {end_id:?}",
                        ))
                    })
                    .map(|(index, _)| index)?;
                let mut begin_note = self.notes.pop_by_idx(begin_idx).ok_or_else(|| {
                    ParseWarning::SyntaxError(format!(
                        "Cannot find begin note for LNOBJ {end_id:?}"
                    ))
                })?;

                let mut begin_note_tuple = begin_note
                    .channel_id
                    .try_into_map::<T>()
                    .ok_or_else(|| {
                        ParseWarning::SyntaxError(format!(
                            "channel of specified note for LNOBJ cannot become LN {end_id:?}"
                        ))
                    })?
                    .as_tuple();
                begin_note_tuple.1 = NoteKind::Long;
                begin_note.channel_id = T::from_tuple(begin_note_tuple).to_channel_id();
                self.notes.push_note(begin_note);

                let mut end_note_tuple = end_note
                    .channel_id
                    .try_into_map::<T>()
                    .ok_or_else(|| {
                        ParseWarning::SyntaxError(format!(
                            "channel of specified note for LNOBJ cannot become LN {end_id:?}"
                        ))
                    })?
                    .as_tuple();
                end_note_tuple.1 = NoteKind::Long;
                end_note.channel_id = T::from_tuple(end_note_tuple).to_channel_id();
                self.notes.push_note(end_note);
            }
            Token::DefExRank(judge_level) => {
                let judge_level = JudgeLevel::OtherInt(*judge_level as i64);
                self.scope_defines.exrank_defs.insert(
                    ObjId::try_from([0, 0]).map_err(|_| {
                        ParseWarning::SyntaxError("Invalid ObjId [0, 0]".to_string())
                    })?,
                    ExRankDef {
                        id: ObjId::try_from([0, 0]).map_err(|_| {
                            ParseWarning::SyntaxError("Invalid ObjId [0, 0]".to_string())
                        })?,
                        judge_level,
                    },
                );
            }
            Token::LnMode(ln_mode_type) => {
                self.header.ln_mode = *ln_mode_type;
            }
            Token::Movie(path) => self.header.movie = Some(path.into()),
            Token::Preview(path) => self.header.preview_music = Some(path.into()),
            #[cfg(feature = "minor-command")]
            Token::Cdda(big_uint) => self.others.cdda.push(big_uint.clone()),
            #[cfg(feature = "minor-command")]
            Token::BaseBpm(generic_decimal) => {
                self.arrangers.base_bpm = Some(generic_decimal.clone());
            }
            Token::NotACommand(line) => self.others.non_command_lines.push(line.to_string()),
            Token::UnknownCommand(line) => self.others.unknown_command_lines.push(line.to_string()),
            Token::Base62 | Token::Charset(_) => {
                // Pass.
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
                return Err(ParseWarning::UnexpectedControlFlow);
            }
            #[cfg(feature = "minor-command")]
            Token::CharFile(path) => {
                self.graphics.char_file = Some(path.into());
            }
            #[cfg(feature = "minor-command")]
            Token::DivideProp(prop) => {
                self.others.divide_prop = Some(prop.to_string());
            }
            #[cfg(feature = "minor-command")]
            Token::Materials(path) => {
                self.others.materials_path = Some(path.to_path_buf());
            }
            #[cfg(feature = "minor-command")]
            Token::VideoColors(colors) => {
                self.graphics.video_colors = Some(*colors);
            }
            #[cfg(feature = "minor-command")]
            Token::VideoDly(delay) => {
                self.graphics.video_dly = Some(delay.clone());
            }
            #[cfg(feature = "minor-command")]
            Token::VideoFs(frame_rate) => {
                self.graphics.video_fs = Some(frame_rate.clone());
            }
        }
        Ok(())
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
        let denominator = NonZeroU64::new(denominator)?;
        let time = ObjTime::new(track.0, i as u64, denominator);
        Some((time, obj))
    })
}

#[cfg(feature = "minor-command")]
fn opacity_from_message(
    track: Track,
    message: &'_ str,
) -> impl Iterator<Item = (ObjTime, u8)> + '_ {
    let denominator = message.len() as u64 / 2;
    let mut chars = message.chars().tuples().enumerate();
    std::iter::from_fn(move || {
        let (i, c1, c2) = loop {
            let (i, (c1, c2)) = chars.next()?;
            if !(c1 == '0' && c2 == '0') {
                break (i, c1, c2);
            }
        };
        // Parse opacity value from hex string
        let opacity_hex = format!("{c1}{c2}");
        let opacity_value = u8::from_str_radix(&opacity_hex, 16).ok()?;
        let denominator = NonZeroU64::new(denominator)?;
        let time = ObjTime::new(track.0, i as u64, denominator);
        Some((time, opacity_value))
    })
}

fn volume_from_message(track: Track, message: &'_ str) -> impl Iterator<Item = (ObjTime, u8)> + '_ {
    let denominator = message.len() as u64 / 2;
    let mut chars = message.chars().tuples().enumerate();
    std::iter::from_fn(move || {
        let (i, c1, c2) = loop {
            let (i, (c1, c2)) = chars.next()?;
            if !(c1 == '0' && c2 == '0') {
                break (i, c1, c2);
            }
        };
        // Parse volume value from hex string
        let volume_hex = format!("{c1}{c2}");
        let volume_value = u8::from_str_radix(&volume_hex, 16).ok()?;
        let denominator = NonZeroU64::new(denominator)?;
        let time = ObjTime::new(track.0, i as u64, denominator);
        Some((time, volume_value))
    })
}

/// Bms Parse Output with AST
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use]
pub struct ParseOutputWithAst<T: KeyLayoutMapper = KeyLayoutBeat> {
    /// The output Bms.
    pub bms: Bms<T>,
    /// Warnings that occurred during AST building.
    pub ast_build_warnings: Vec<AstBuildWarningWithRange>,
    /// Warnings that occurred during AST parsing (RNG execution stage).
    pub ast_parse_warnings: Vec<AstParseWarningWithRange>,
    /// Warnings that occurred during parsing.
    pub parse_warnings: Vec<ParseWarningWithRange>,
}

impl<T: KeyLayoutMapper> Bms<T> {
    /// Parses a token stream into [`Bms`] with AST.
    pub fn from_token_stream_with_ast<'a>(
        token_iter: impl IntoIterator<Item = &'a TokenWithRange<'a>>,
        rng: impl Rng,
        prompt_handler: impl PromptHandler,
    ) -> ParseOutputWithAst<T> {
        let AstBuildOutput {
            root,
            ast_build_warnings,
        } = AstRoot::from_token_stream(token_iter);
        let (AstParseOutput { token_refs }, ast_parse_warnings) = root.parse_with_warnings(rng);
        let ParseOutput {
            bms,
            parse_warnings,
        } = Self::from_token_stream(token_refs, prompt_handler);
        ParseOutputWithAst {
            bms,
            ast_build_warnings,
            ast_parse_warnings,
            parse_warnings,
        }
    }
}

impl ToAriadne for ParseWarningWithRange {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        let (start, end) = self.as_span();
        let filename = src.name().to_string();
        Report::build(ReportKind::Warning, (filename.clone(), start..end))
            .with_message("parse: ".to_string() + &self.content().to_string())
            .with_label(Label::new((filename, start..end)).with_color(Color::Blue))
            .finish()
    }
}
