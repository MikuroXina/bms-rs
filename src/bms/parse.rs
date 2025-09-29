//! Parsing Bms from [`TokenStream`].
//!
//! Raw [String] == [lex] ==> [`TokenStream`] (in [`BmsLexOutput`]) == [parse] ==> [Bms] (in
//! [`BmsParseOutput`])

pub mod check_playing;
pub mod prompt;
pub mod token_processor;
pub mod validity;

use std::{borrow::Cow, num::NonZeroU64};

use itertools::Itertools;
use thiserror::Error;

use crate::diagnostics::{SimpleSource, ToAriadne};
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

use self::prompt::Prompter;

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
        prompt_handler: impl Prompter,
    ) -> ParseOutput<T> {
        let mut bms = Self::default();
        let mut parse_warnings: Vec<ParseWarningWithRange> = vec![];
        for token in token_iter {
            let mut parse_warnings_buf: Vec<ParseWarning> = vec![];
            let parse_result = bms.parse(token, &prompt_handler, &mut parse_warnings_buf);
            parse_warnings.extend(
                parse_warnings_buf
                    .into_iter()
                    .map(|error| error.into_wrapper(token)),
            );
            if let Err(error) = parse_result {
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
        prompt_handler: &impl Prompter,
        parse_warnings: &mut Vec<ParseWarning>,
    ) -> Result<()> {
        match token.content() {
            Token::Artist(artist) => self.header.artist = Some(artist.to_string()),
            Token::Banner(file) => self.header.banner = Some(file.into()),
            Token::BackBmp(bmp) => self.header.back_bmp = Some(bmp.into()),
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
            Token::Rank(rank) => self.header.rank = Some(*rank),
            Token::StageFile(file) => self.header.stage_file = Some(file.into()),
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
            #[cfg(feature = "minor-command")]
            Token::Message {
                track,
                channel: Channel::ChangeOption,
                message,
            } => {
                for (_time, obj) in ids_from_message(*track, message, |w| parse_warnings.push(w)) {
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
                channel: Channel::BgmVolume,
                message,
            } => {
                for (time, volume_value) in
                    hex_values_from_message(*track, message, |w| parse_warnings.push(w))
                {
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
                for (time, volume_value) in
                    hex_values_from_message(*track, message, |w| parse_warnings.push(w))
                {
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
                channel: Channel::Seek,
                message,
            } => {
                for (time, seek_id) in ids_from_message(*track, message, |w| parse_warnings.push(w))
                {
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
                for (time, text_id) in ids_from_message(*track, message, |w| parse_warnings.push(w))
                {
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
                for (time, judge_id) in
                    ids_from_message(*track, message, |w| parse_warnings.push(w))
                {
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
                channel: Channel::Option,
                message,
            } => {
                for (time, option_id) in
                    ids_from_message(*track, message, |w| parse_warnings.push(w))
                {
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
            Token::DefExRank(judge_level) => {
                let judge_level = JudgeLevel::OtherInt(*judge_level as i64);
                self.scope_defines.exrank_defs.insert(
                    ObjId::try_from([b'0', b'0']).map_err(|_| {
                        ParseWarning::SyntaxError("Invalid ObjId [0, 0]".to_string())
                    })?,
                    ExRankDef {
                        id: ObjId::try_from([b'0', b'0']).map_err(|_| {
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

/// Parses message values with warnings.
///
/// This function processes BMS message strings by filtering out invalid characters,
/// then parsing character pairs into values using the provided `parse_value` function.
/// It returns an iterator that yields `(ObjTime, T)` pairs for each successfully parsed value.
///
/// # Arguments
/// * `track` - The track number for time calculation
/// * `message` - The raw message string to parse
/// * `parse_value` - A closure that takes two characters and a mutable warnings vector,
///   returning `Option<T>` if parsing succeeds or `None` if the pair should be skipped
/// * `parse_warnings` - A mutable vector to collect parsing warnings
///
/// # Returns
/// An iterator yielding `(ObjTime, T)` pairs where:
/// - `ObjTime` represents the timing position within the track
/// - `T` is the parsed value from character pairs
///
/// # Behavior
/// - Messages are first filtered to remove invalid characters
/// - Character pairs are processed sequentially
/// - Empty pairs ('00') are typically skipped by the parse_value function
/// - Time calculation uses the track number and pair index as numerator,
///   with total pair count as denominator
/// - Length validation ensures message length is at least 2 characters
fn parse_message_values_with_warnings<'a, T, F>(
    track: Track,
    message: &'a str,
    mut parse_value: F,
    mut push_parse_warning: impl FnMut(ParseWarning) + 'a,
) -> impl Iterator<Item = (ObjTime, T)> + 'a
where
    F: FnMut(char, char) -> Option<Result<T>> + 'a,
{
    // Centralize message filtering here so callers don't need to call `filter_message`.
    // Use a simple pair-wise char reader without storing self-referential iterators.

    // Filter the message to remove invalid characters
    let filtered = filter_message(message);

    // Convert the filtered string to a vector of characters for pair-wise processing
    let chars: Vec<char> = filtered.chars().collect();

    // Calculate the denominator for time calculation (total number of character pairs)
    // This will be None if the message length is less than 2
    let denominator_opt = NonZeroU64::new((chars.len() / 2) as u64);

    // Create an iterator that yields character pairs from the filtered message
    let mut pairs_iter = chars.into_iter().tuples::<(char, char)>();

    // Track the current pair index for time calculation
    let mut pair_index: u64 = 0;

    std::iter::from_fn(move || {
        // Ensure we have a valid denominator (at least 2 characters in original message)
        let Some(denominator) = denominator_opt else {
            // Emit a warning for invalid message length
            push_parse_warning(ParseWarning::SyntaxError(
                "message length must be greater than or equals to 2".to_string(),
            ));
            return None;
        };

        loop {
            // Get the next character pair, or end iteration if none remain
            let (c1, c2) = pairs_iter.next()?;

            // Store current pair index before incrementing
            let current_index = pair_index;
            pair_index += 1;

            // Try to parse the character pair using the provided parse_value function
            match parse_value(c1, c2) {
                Some(Ok(value)) => {
                    // Successfully parsed a value, calculate its timing position
                    let time = ObjTime::new(track.0, current_index, denominator);
                    return Some((time, value));
                }
                Some(Err(warning)) => {
                    // Push the warning and continue to the next pair
                    push_parse_warning(warning);
                }
                None => {
                    // Skip this value, don't report a warning
                    continue;
                }
            }
        }
    })
}

fn ids_from_message<'a>(
    track: Track,
    message: &'a str,
    push_parse_warning: impl FnMut(ParseWarning) + 'a,
) -> impl Iterator<Item = (ObjTime, ObjId)> + 'a {
    parse_message_values_with_warnings(
        track,
        message,
        |c1, c2| {
            if c1 == '0' && c2 == '0' {
                return None;
            }
            Some(match ObjId::try_from([c1, c2]) {
                Ok(obj) => Ok(obj),
                Err(_) => Err(ParseWarning::SyntaxError(format!(
                    "Invalid object id digits: {c1}{c2}"
                ))),
            })
        },
        push_parse_warning,
    )
}

// Unified hex pair parser for message channels emitting u8 values
fn hex_values_from_message<'a>(
    track: Track,
    message: &'a str,
    push_parse_warning: impl FnMut(ParseWarning) + 'a,
) -> impl Iterator<Item = (ObjTime, u8)> + 'a {
    parse_message_values_with_warnings(
        track,
        message,
        |c1, c2| {
            if c1 == '0' && c2 == '0' {
                return None;
            }
            Some(match u8::from_str_radix(&format!("{c1}{c2}"), 16) {
                Ok(v) => Ok(v),
                Err(_) => Err(ParseWarning::SyntaxError(format!(
                    "Invalid hex digits: {c1}{c2}"
                ))),
            })
        },
        push_parse_warning,
    )
}

fn filter_message(message: &str) -> Cow<'_, str> {
    let result = message
        .chars()
        .try_fold(String::with_capacity(message.len()), |mut acc, ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '.' {
                acc.push(ch);
                Ok(acc)
            } else {
                Err(acc)
            }
        });
    match result {
        Ok(_) => Cow::Borrowed(message),
        Err(filtered) => Cow::Owned(filtered),
    }
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
        prompt_handler: impl Prompter,
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
