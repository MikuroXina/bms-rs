//! This module provides [`TokenProcessor`] and its implementations, which reads [`Token`] and applies data to [`Bms`].
//!
//! Also it provides preset functions that returns a [`TokenProcessor`] trait object:
//!
//! - [`common_preset`] - Commonly used processors.
//! - [`minor_preset`] - All of processors this crate provided.

use std::{borrow::Cow, cell::RefCell, rc::Rc};

use itertools::Itertools;

use crate::bms::{
    parse::{ParseError, ParseErrorWithRange, ParseWarningWithRange},
    prelude::*,
};

mod bmp;
mod bpm;
mod identity;
mod judge;
mod metadata;
mod music_info;
mod option;
mod random;
mod repr;
mod resources;
mod scroll;
mod section_len;
mod speed;
mod sprite;
mod stop;
mod text;
mod video;
mod volume;
mod wav;

/// A checkpoint of input position, allowing temporary rewinds/restores.
pub struct Checkpoint<'a, 't>(pub &'a [&'t TokenWithRange<'t>]);

/// Processing context passed through token processors.
///
/// Contains the current input view, the prompter, and collected warnings.
pub struct ProcessContext<'a, 't, P> {
    /// The mutable view of remaining tokens to be processed.
    input: &'a mut &'a [&'t TokenWithRange<'t>],
    /// The prompter used to handle duplications and user-facing prompts.
    prompter: &'a P,
    /// Collected warnings (with source ranges) produced during processing.
    reported: Vec<ParseWarningWithRange>,
}

impl<'a, 't, P> ProcessContext<'a, 't, P> {
    /// Creates a new processing context from a token slice view and a prompter.
    pub const fn new(input: &'a mut &'a [&'t TokenWithRange<'t>], prompter: &'a P) -> Self {
        Self {
            input,
            prompter,
            reported: Vec::new(),
        }
    }

    /// Saves the current input position to a checkpoint.
    #[must_use] 
    pub const fn save(&self) -> Checkpoint<'a, 't> {
        Checkpoint(self.input)
    }

    /// Restores the input position from a previously saved checkpoint.
    pub const fn restore(&mut self, checkpoint: Checkpoint<'a, 't>) {
        *self.input = checkpoint.0;
    }

    /// Returns a shared reference to the prompter.
    #[must_use] 
    pub const fn prompter(&self) -> &P { self.prompter }

    

    /// Takes current input view and consumes it (resets to empty).
    pub const fn take_input(&mut self) -> &'a [&'t TokenWithRange<'t>] {
        let view = *self.input;
        *self.input = &[];
        view
    }

    /// Records a warning produced during token processing.
    pub fn warn(&mut self, warning: ParseWarningWithRange) {
        self.reported.push(warning);
    }

    /// Consumes the context and returns collected warnings.
    #[must_use] 
    pub fn into_warnings(self) -> Vec<ParseWarningWithRange> {
        self.reported
    }

    /// Iterates over all remaining tokens and collects warnings from the handler.
    pub fn all_tokens<F, I>(&mut self, mut f: F) -> Result<(), ParseErrorWithRange>
    where
        F: FnMut(&'a TokenWithRange<'t>, &P) -> Result<I, ParseError>,
        I: IntoIterator<Item = ParseWarningWithRange>,
    {
        let view = self.take_input();
        let prompter = self.prompter;
        for token in view.iter().copied() {
            let warns = f(token, prompter).map_err(|e| e.into_wrapper(token))?;
            self.reported.extend(warns);
        }
        Ok(())
    }
}

/// A processor of tokens in the BMS. An implementation takes control only one feature about definitions and placements such as `WAVxx` definition and its sound object.
pub trait TokenProcessor {
    /// A result data of the process.
    type Output;

    /// Processes commands by consuming all the stream `input`. It mutates `input`
    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> Result<Self::Output, ParseErrorWithRange>;

    /// Creates a processor [`SequentialProcessor`] which does `self` then `second`.
    fn then<S>(self, second: S) -> SequentialProcessor<Self, S>
    where
        Self: Sized,
        S: TokenProcessor + Sized,
    {
        SequentialProcessor {
            first: self,
            second,
        }
    }

    /// Maps a result of the processor by the mapping function `f`.
    fn map<F, O>(self, f: F) -> Mapped<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Output) -> O,
    {
        Mapped {
            source: self,
            mapping: f,
        }
    }
}

impl<T: TokenProcessor + ?Sized> TokenProcessor for Box<T> {
    type Output = <T as TokenProcessor>::Output;

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> Result<Self::Output, ParseErrorWithRange> {
        T::process(self, ctx)
    }
}

/// A processor [`SequentialProcessor`] which does `first` then `second`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SequentialProcessor<F, S> {
    first: F,
    second: S,
}

impl<F, S> TokenProcessor for SequentialProcessor<F, S>
where
    F: TokenProcessor,
    S: TokenProcessor,
{
    type Output = (F::Output, S::Output);

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> Result<Self::Output, ParseErrorWithRange> {
        // Create isolated contexts that share the same input view, avoiding borrow conflicts.
        let original_input = *ctx.input;

        // First pass context borrows the same prompter to avoid cloning.
        let mut first_input = original_input;
        let mut first_ctx = ProcessContext {
            input: &mut first_input,
            prompter: ctx.prompter(),
            reported: Vec::new(),
        };
        let first_res = self.first.process(&mut first_ctx);

        match first_res {
            Ok(first_output) => {
                // Second pass context also starts from the original input view.
                let mut second_input = original_input;
                let mut second_ctx = ProcessContext {
                    input: &mut second_input,
                    prompter: ctx.prompter(),
                    reported: Vec::new(),
                };
                let second_output = self.second.process(&mut second_ctx)?;
                let mut merged_reported = core::mem::take(&mut first_ctx.reported);
                let second_reported = core::mem::take(&mut second_ctx.reported);
                drop(first_ctx);
                drop(second_ctx);
                merged_reported.extend(second_reported);
                ctx.reported.extend(merged_reported);
                Ok((first_output, second_output))
            }
            Err(err) => Err(err),
        }
    }
}

/// A processor [`SequentialProcessor`] which maps the output of the token processor `TP` by the function `F`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Mapped<TP, F> {
    source: TP,
    mapping: F,
}

impl<O, TP, F> TokenProcessor for Mapped<TP, F>
where
    TP: TokenProcessor,
    F: Fn(TP::Output) -> O,
{
    type Output = O;

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> Result<Self::Output, ParseErrorWithRange> {
        let res = self.source.process(ctx)?;
        Ok((self.mapping)(res))
    }
}

/// Returns commonly used processors.
pub(crate) fn common_preset<T: KeyLayoutMapper, R: Rng>(
    rng: Rc<RefCell<R>>,
    relaxed: bool,
) -> impl TokenProcessor<Output = Bms> {
    let case_sensitive_obj_id = Rc::new(RefCell::new(false));
    let sub_processor = repr::RepresentationProcessor::new(&case_sensitive_obj_id)
        .then(bmp::BmpProcessor::new(&case_sensitive_obj_id))
        .then(bpm::BpmProcessor::new(&case_sensitive_obj_id))
        .then(judge::JudgeProcessor::new(&case_sensitive_obj_id))
        .then(metadata::MetadataProcessor)
        .then(music_info::MusicInfoProcessor)
        .then(scroll::ScrollProcessor::new(&case_sensitive_obj_id))
        .then(section_len::SectionLenProcessor)
        .then(speed::SpeedProcessor::new(&case_sensitive_obj_id))
        .then(sprite::SpriteProcessor)
        .then(stop::StopProcessor::new(&case_sensitive_obj_id))
        .then(video::VideoProcessor::new(&case_sensitive_obj_id))
        .then(wav::WavProcessor::<T>::new(&case_sensitive_obj_id));
    random::RandomTokenProcessor::new(rng, sub_processor, relaxed).map(
        |(
            (
                (
                    (
                        (
                            (
                                ((((((repr, bmp), bpm), judge), metadata), music_info), scroll),
                                section_len,
                            ),
                            speed,
                        ),
                        sprite,
                    ),
                    stop,
                ),
                video,
            ),
            wav,
        )| {
            Bms {
                bmp,
                bpm,
                judge,
                metadata,
                music_info,

                option: Default::default(),
                repr,

                resources: Default::default(),
                scroll,
                section_len,
                speed,
                sprite,
                stop,
                text: Default::default(),
                video,
                volume: Default::default(),
                wav,
            }
        },
    )
}

/// Returns all of processors this crate provided.
pub(crate) fn minor_preset<T: KeyLayoutMapper, R: Rng>(
    rng: Rc<RefCell<R>>,
    relaxed: bool,
) -> impl TokenProcessor<Output = Bms> {
    let case_sensitive_obj_id = Rc::new(RefCell::new(false));
    let sub_processor = repr::RepresentationProcessor::new(&case_sensitive_obj_id)
        .then(bmp::BmpProcessor::new(&case_sensitive_obj_id))
        .then(bpm::BpmProcessor::new(&case_sensitive_obj_id))
        .then(judge::JudgeProcessor::new(&case_sensitive_obj_id))
        .then(metadata::MetadataProcessor)
        .then(music_info::MusicInfoProcessor);

    let sub_processor = sub_processor
        .then(option::OptionProcessor::new(&case_sensitive_obj_id))
        .then(resources::ResourcesProcessor);
    let sub_processor = sub_processor
        .then(scroll::ScrollProcessor::new(&case_sensitive_obj_id))
        .then(section_len::SectionLenProcessor)
        .then(speed::SpeedProcessor::new(&case_sensitive_obj_id))
        .then(sprite::SpriteProcessor)
        .then(stop::StopProcessor::new(&case_sensitive_obj_id))
        .then(text::TextProcessor::new(&case_sensitive_obj_id))
        .then(video::VideoProcessor::new(&case_sensitive_obj_id))
        .then(volume::VolumeProcessor)
        .then(wav::WavProcessor::<T>::new(&case_sensitive_obj_id));
    random::RandomTokenProcessor::new(rng, sub_processor, relaxed).map(
        |(
            (
                (
                    (
                        (
                            (
                                (
                                    (
                                        (
                                            (
                                                (
                                                    (
                                                        ((((repr, bmp), bpm), judge), metadata),
                                                        music_info,
                                                    ),
                                                    option,
                                                ),
                                                resources,
                                            ),
                                            scroll,
                                        ),
                                        section_len,
                                    ),
                                    speed,
                                ),
                                sprite,
                            ),
                            stop,
                        ),
                        text,
                    ),
                    video,
                ),
                volume,
            ),
            wav,
        )| Bms {
            bmp,
            bpm,
            judge,
            metadata,
            music_info,
            option,
            repr,
            resources,
            scroll,
            section_len,
            speed,
            sprite,
            stop,
            text,
            video,
            volume,
            wav,
        },
    )
}

fn parse_obj_ids(
    track: Track,
    message: SourceRangeMixin<&str>,
    case_sensitive_obj_id: &RefCell<bool>,
) -> (Vec<(ObjTime, ObjId)>, Vec<ParseWarningWithRange>) {
    let mut warnings = Vec::new();
    if !message.content().len().is_multiple_of(2) {
        warnings.push(
            ParseWarning::SyntaxError("expected 2-digit object ids".into()).into_wrapper(&message),
        );
    }

    let denom = message.content().len() as u64 / 2;
    let messages = message
        .content()
        .chars()
        .tuples()
        .enumerate()
        .filter_map(|(i, (c1, c2))| {
            let arr: [char; 2] = (c1, c2).into();
            let buf = arr.into_iter().collect::<String>();
            match ObjId::try_from(&buf, *case_sensitive_obj_id.borrow()) {
                Ok(id) if id.is_null() => None,
                Ok(id) => ObjTime::new(track.0, i as u64, denom).map(|time| (time, id)),
                Err(warning) => {
                    warnings.push(warning.into_wrapper(&message));
                    None
                }
            }
        });
    (messages.collect(), warnings)
}

fn parse_hex_values(
    track: Track,
    message: SourceRangeMixin<&str>,
) -> (Vec<(ObjTime, u8)>, Vec<ParseWarningWithRange>) {
    let mut warnings = Vec::new();
    if !message.content().len().is_multiple_of(2) {
        warnings.push(
            ParseWarning::SyntaxError("expected 2-digit hex values".into()).into_wrapper(&message),
        );
    }

    let denom = message.content().len() as u64 / 2;
    let message = message
        .content()
        .chars()
        .tuples()
        .enumerate()
        .filter_map(|(i, (c1, c2))| {
            let arr: [char; 2] = (c1, c2).into();
            let buf = arr.into_iter().collect::<String>();
            u8::from_str_radix(&buf, 16).map_or_else(
                |_| {
                    warnings.push(
                        ParseWarning::SyntaxError(format!("invalid hex digits ({buf:?}"))
                            .into_wrapper(&message),
                    );
                    None
                },
                |value| ObjTime::new(track.0, i as u64, denom).map(|time| (time, value)),
            )
        });
    (message.collect(), warnings)
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
