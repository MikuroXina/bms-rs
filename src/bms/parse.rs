//! Parsing Bms from [`TokenStream`].
//!
//! Raw [String] == [lex] ==> [`TokenStream`] (in [`BmsLexOutput`]) == [parse] ==> [Bms] (in
//! [`BmsParseOutput`])

pub mod check_playing;
pub mod prompt;
pub mod token_processor;
pub mod validity;
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
            Token::Comment(comment) => self
                .header
                .comment
                .get_or_insert_with(Vec::new)
                .push(comment.to_string()),
            Token::Difficulty(diff) => self.header.difficulty = Some(*diff),
            Token::Email(email) => self.header.email = Some(email.to_string()),
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
            Token::PathWav(wav_path_root) => self.notes.wav_path_root = Some(wav_path_root.into()),
            Token::Player(player) => self.header.player = Some(*player),
            Token::PlayLevel(play_level) => self.header.play_level = Some(*play_level),
            Token::StageFile(file) => self.header.stage_file = Some(file.into()),
            Token::SubArtist(sub_artist) => self.header.sub_artist = Some(sub_artist.to_string()),
            Token::SubTitle(subtitle) => self.header.subtitle = Some(subtitle.to_string()),
            Token::Title(title) => self.header.title = Some(title.to_string()),
            Token::Total(total) => {
                self.header.total = Some(total.clone());
            }
            Token::Url(url) => self.header.url = Some(url.to_string()),
            Token::VideoFile(video_file) => self.graphics.video_file = Some(video_file.into()),
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
