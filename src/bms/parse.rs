//! Parsing Bms from [`TokenStream`].
//!
//! Raw [String] == [lex] ==> [`TokenStream`] (in [`BmsLexOutput`]) == [parse] ==> [Bms] (in
//! [`BmsParseOutput`])

pub mod check_playing;
pub mod prompt;
pub mod token_processor;
pub mod validity;
use std::{cell::RefCell, rc::Rc};

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
        channel::{Channel, mapper::KeyLayoutMapper},
        mixin::SourceRangeMixin,
        time::{ObjTime, Track},
    },
    lex::token::{Token, TokenWithRange},
    model::Bms,
};

use self::{prompt::Prompter, token_processor::minor_preset};

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
    /// Failed to convert a byte into a base-62 character `0-9A-Za-z`.
    #[error("expected id format is base 62 (`0-9A-Za-z`)")]
    OutOfBase62,
}

/// Type alias of `core::result::Result<T, ParseWarning>`
pub(crate) type Result<T> = core::result::Result<T, ParseWarning>;

/// A parse warning with position information.
pub type ParseWarningWithRange = SourceRangeMixin<ParseWarning>;

/// Bms Parse Output
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use]
pub struct ParseOutput {
    /// The output Bms.
    pub bms: Bms,
    /// Warnings that occurred during parsing.
    pub parse_warnings: Vec<ParseWarningWithRange>,
}

impl Bms {
    /// Parses a token stream into [`Bms`] without AST.
    pub fn from_token_stream<'a, T: KeyLayoutMapper, P: Prompter>(
        token_iter: impl IntoIterator<Item = &'a TokenWithRange<'a>>,
        prompt_handler: P,
    ) -> ParseOutput {
        let bms = Self::default();
        let share = Rc::new(RefCell::new(bms));
        let mut comments = vec![];
        let mut headers = vec![];
        let mut messages = vec![];
        for token in token_iter {
            match token.content() {
                Token::NotACommand(line) => comments.push((token.range(), line)),
                Token::Header { name, args } => {
                    headers.push((token.range(), name, args));
                }
                Token::Message {
                    track,
                    channel,
                    message,
                } => {
                    messages.push((token.range(), track, channel, message));
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
                    // control tokens skipped
                }
            }
        }
        let preset = minor_preset::<P, T>(Rc::clone(&share), &prompt_handler);
        let mut parse_warnings: Vec<ParseWarningWithRange> = vec![];
        for (range, comment) in &comments {
            for proc in &preset {
                match proc.on_comment(comment) {
                    std::ops::ControlFlow::Continue(()) => continue,
                    std::ops::ControlFlow::Break(Ok(())) => break,
                    std::ops::ControlFlow::Break(Err(err)) => {
                        parse_warnings.push(err.into_wrapper_range((*range).clone()));
                        break;
                    }
                }
            }
        }
        // Two-pass header processing:
        // 1) Run representation processor first across all headers to record raw lines and set global flags (e.g., BASE 62).
        // 2) Run remaining processors on headers, now with representation effects applied.
        let mut skip_header_in_second_pass = vec![false; headers.len()];
        if let Some(repr_proc) = preset.get(0) {
            for (idx, (range, name, args)) in headers.iter().enumerate() {
                match repr_proc.on_header(name, args.as_ref()) {
                    std::ops::ControlFlow::Continue(()) => {}
                    std::ops::ControlFlow::Break(Ok(())) => {
                        // Skip running other processors for this header in second pass to preserve original short-circuit behavior.
                        skip_header_in_second_pass[idx] = true;
                    }
                    std::ops::ControlFlow::Break(Err(err)) => {
                        parse_warnings.push(err.into_wrapper_range((*range).clone()));
                        skip_header_in_second_pass[idx] = true;
                    }
                }
            }
        }

        // Second pass: iterate processors first, then headers, to preserve a stable processor-priority order
        // for warnings/emissions while still short-circuiting headers once handled.
        let mut header_already_handled = skip_header_in_second_pass;
        for proc in preset.iter().skip(1) {
            for (idx, (range, name, args)) in headers.iter().enumerate() {
                if header_already_handled[idx] {
                    continue;
                }
                match proc.on_header(name, args.as_ref()) {
                    std::ops::ControlFlow::Continue(()) => {}
                    std::ops::ControlFlow::Break(Ok(())) => {
                        header_already_handled[idx] = true;
                    }
                    std::ops::ControlFlow::Break(Err(err)) => {
                        parse_warnings.push(err.into_wrapper_range((*range).clone()));
                        header_already_handled[idx] = true;
                    }
                }
            }
        }
        for (range, track, channel, message) in &messages {
            for proc in &preset {
                match proc.on_message(**track, **channel, message.as_ref()) {
                    std::ops::ControlFlow::Continue(()) => continue,
                    std::ops::ControlFlow::Break(Ok(())) => break,
                    std::ops::ControlFlow::Break(Err(err)) => {
                        parse_warnings.push(err.into_wrapper_range((*range).clone()));
                        break;
                    }
                }
            }
        }
        std::mem::drop(preset);

        ParseOutput {
            bms: Rc::into_inner(share)
                .expect("processors must be dropped")
                .into_inner(),
            parse_warnings,
        }
    }
}

/// Bms Parse Output with AST
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use]
pub struct ParseOutputWithAst {
    /// The output Bms.
    pub bms: Bms,
    /// Warnings that occurred during AST building.
    pub ast_build_warnings: Vec<AstBuildWarningWithRange>,
    /// Warnings that occurred during AST parsing (RNG execution stage).
    pub ast_parse_warnings: Vec<AstParseWarningWithRange>,
    /// Warnings that occurred during parsing.
    pub parse_warnings: Vec<ParseWarningWithRange>,
}

impl Bms {
    /// Parses a token stream into [`Bms`] with AST.
    pub fn from_token_stream_with_ast<'a, T: KeyLayoutMapper, P: Prompter>(
        token_iter: impl IntoIterator<Item = &'a TokenWithRange<'a>>,
        rng: impl Rng,
        prompt_handler: P,
    ) -> ParseOutputWithAst {
        let AstBuildOutput {
            root,
            ast_build_warnings,
        } = AstRoot::from_token_stream(token_iter);
        let (AstParseOutput { token_refs }, ast_parse_warnings) = root.parse_with_warnings(rng);
        let ParseOutput {
            bms,
            parse_warnings,
        } = Self::from_token_stream::<'a, T, P>(token_refs, prompt_handler);
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
