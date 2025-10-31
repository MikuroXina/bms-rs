//! This module provides [`TokenProcessor`] and its implementations, which reads [`Token`] and applies data to [`Bms`].
//!
//! Also it provides preset functions that returns a [`TokenProcessor`] trait object:
//!
//! - [`common_preset`] - Commonly used processors.
//! - [`minor_preset`] - All of processors this crate provided.

use std::{borrow::Cow, cell::RefCell, num::NonZeroU64, rc::Rc};

use itertools::Itertools;

use crate::bms::{error::ControlFlowWarning, prelude::*};

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

/// A processor of tokens in the BMS. An implementation takes control only one feature about definitions and placements such as `WAVxx` definition and its sound object.
pub trait TokenProcessor {
    /// A result data of the process.
    type Output;

    /// Processes commands by consuming all the stream `input`. It mutates `input`
    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> (Self::Output, Vec<ParseWarningWithRange>);

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

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> (Self::Output, Vec<ParseWarningWithRange>) {
        T::process(self, input, prompter)
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

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> (Self::Output, Vec<ParseWarningWithRange>) {
        let mut cloned = *input;
        let (first_output, mut first_warnings) = self.first.process(&mut cloned, prompter);
        let (second_output, second_warnings) = self.second.process(input, prompter);
        first_warnings.extend(second_warnings);
        ((first_output, second_output), first_warnings)
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

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> (Self::Output, Vec<ParseWarningWithRange>) {
        let (res, warnings) = self.source.process(input, prompter);
        ((self.mapping)(res), warnings)
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

fn all_tokens<
    'a,
    P: Prompter,
    F: FnMut(&'a Token<'_>) -> Result<Option<ParseWarning>, ControlFlowWarning>,
>(
    input: &mut &'a [&TokenWithRange<'_>],
    _prompter: &P,
    mut f: F,
) -> ((), Vec<ParseWarningWithRange>) {
    let mut warnings = Vec::new();

    for token in &**input {
        match f(token.content()) {
            Ok(Some(warning)) => {
                let warning_with_range = warning.into_wrapper(token);
                warnings.push(warning_with_range);
            }
            Ok(None) => {}
            Err(error) => {
                let error_with_range = ParseWarning::from(error).into_wrapper(token);
                warnings.push(error_with_range);
            }
        }
    }
    *input = &[];
    ((), warnings)
}

fn all_tokens_with_range<
    'a,
    P: Prompter,
    F: FnMut(&'a TokenWithRange<'_>) -> Result<Option<ParseWarning>, ControlFlowWarning>,
>(
    input: &mut &'a [&TokenWithRange<'_>],
    _prompter: &P,
    mut f: F,
) -> ((), Vec<ParseWarningWithRange>) {
    let mut warnings = Vec::new();

    for token in &**input {
        match f(token) {
            Ok(Some(warning)) => {
                let warning_with_range = warning.into_wrapper(token);
                warnings.push(warning_with_range);
            }
            Ok(None) => {}
            Err(error) => {
                let error_with_range = ParseWarning::from(error).into_wrapper(token);
                warnings.push(error_with_range);
            }
        }
    }
    *input = &[];
    ((), warnings)
}

fn parse_obj_ids_with_warnings<P: Prompter>(
    track: Track,
    message: SourceRangeMixin<&str>,
    _prompter: &P,
    case_sensitive_obj_id: &RefCell<bool>,
) -> (Vec<(ObjTime, ObjId)>, Vec<ParseWarningWithRange>) {
    let mut warnings = Vec::new();
    let mut results = Vec::new();

    // Check for non-multiple-of-2 message length
    if !message.content().len().is_multiple_of(2) {
        warnings.push(
            ParseWarning::SyntaxError("expected 2-digit object ids".into()).into_wrapper(&message),
        );
    }

    let denom_opt = NonZeroU64::new(message.content().len() as u64 / 2);
    for (i, (c1, c2)) in message.content().chars().tuples().enumerate() {
        let buf = String::from_iter(<[char; 2]>::from((c1, c2)));
        match ObjId::try_from(&buf, *case_sensitive_obj_id.borrow()) {
            Ok(id) if id.is_null() => {}
            Ok(id) => results.push((
                ObjTime::new(
                    track.0,
                    i as u64,
                    denom_opt.expect("len / 2 won't be zero on reading tuples"),
                ),
                id,
            )),
            Err(warning) => {
                warnings.push(warning.into_wrapper(&message));
            }
        }
    }

    (results, warnings)
}

fn parse_hex_values_with_warnings<P: Prompter>(
    track: Track,
    message: SourceRangeMixin<&str>,
    _prompter: &P,
) -> (Vec<(ObjTime, u8)>, Vec<ParseWarningWithRange>) {
    let mut warnings = Vec::new();
    let mut results = Vec::new();

    // Check for non-multiple-of-2 message length
    if !message.content().len().is_multiple_of(2) {
        warnings.push(
            ParseWarning::SyntaxError("expected 2-digit hex values".into()).into_wrapper(&message),
        );
    }

    let denom_opt = NonZeroU64::new(message.content().len() as u64 / 2);
    for (i, (c1, c2)) in message.content().chars().tuples().enumerate() {
        let buf = String::from_iter(<[char; 2]>::from((c1, c2)));
        match u8::from_str_radix(&buf, 16) {
            Ok(value) => results.push((
                ObjTime::new(
                    track.0,
                    i as u64,
                    denom_opt.expect("len / 2 won't be zero on reading tuples"),
                ),
                value,
            )),
            Err(_) => {
                warnings.push(
                    ParseWarning::SyntaxError(format!("invalid hex digits ({buf:?}"))
                        .into_wrapper(&message),
                );
            }
        }
    }

    (results, warnings)
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
