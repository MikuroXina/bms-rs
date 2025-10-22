//! This module provides [`TokenProcessor`] and its implementations, which reads [`Token`] and applies data to [`Bms`].
//!
//! Also it provides preset functions that returns a [`TokenProcessor`] trait object:
//!
//! - [`pedantic_preset`] - All processors without obsolete/deprecated.
//! - [`common_preset`] - Commonly used processors.
//! - [`minor_preset`] - All of processors this crate provided.

use std::{borrow::Cow, cell::RefCell, marker::PhantomData, num::NonZeroU64, rc::Rc};

use itertools::Itertools;

use super::{ParseError, ParseErrorWithRange, ParseWarning};
use crate::bms::prelude::*;

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

/// A type alias of `Result<(), Vec<ParseWarningWithRange>`.
pub type TokenProcessorResult = Result<Vec<ParseWarningWithRange>, ParseErrorWithRange>;

/// A processor of tokens in the BMS. An implementation takes control only one feature about definitions and placements such as `WAVxx` definition and its sound object.
pub trait TokenProcessor {
    /// Processes commands by consuming all the stream `input`. It mutates `input`
    fn process(&self, input: &mut &[TokenWithRange<'_>]) -> TokenProcessorResult;

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
}

impl<T: TokenProcessor + ?Sized> TokenProcessor for Box<T> {
    fn process(&self, tokens: &mut &[TokenWithRange<'_>]) -> TokenProcessorResult {
        T::process(self, tokens)
    }
}

/// A processor [`SequentialProcessor`] which does `first` then `second`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SequentialProcessor<F, S> {
    first: F,
    second: S,
}

impl<F: TokenProcessor, S: TokenProcessor> TokenProcessor for SequentialProcessor<F, S> {
    fn process(&self, input: &mut &[TokenWithRange<'_>]) -> TokenProcessorResult {
        let mut cloned = *input;
        let mut warnings = self.first.process(&mut cloned)?;
        warnings.extend(self.second.process(input)?);
        Ok(warnings)
    }
}

/// Returns all processors without obsolete/deprecated.
pub fn pedantic_preset<'a, P: Prompter, T: KeyLayoutMapper + 'a, R: Rng + 'a>(
    bms: Rc<RefCell<Bms>>,
    prompter: &'a P,
    rng: Rc<RefCell<R>>,
) -> impl TokenProcessor + 'a {
    let sub_processor = repr::RepresentationProcessor(Rc::clone(&bms))
        .then(bmp::BmpProcessor(Rc::clone(&bms), prompter))
        .then(bpm::BpmProcessor(Rc::clone(&bms), prompter))
        .then(judge::JudgeProcessor(Rc::clone(&bms), prompter))
        .then(metadata::MetadataProcessor(Rc::clone(&bms)))
        .then(music_info::MusicInfoProcessor(Rc::clone(&bms)));
    let sub_processor = if cfg!(feature = "minor-command") {
        sub_processor
            .then(Box::new(option::OptionProcessor(Rc::clone(&bms), prompter))
                as Box<dyn TokenProcessor>)
    } else {
        sub_processor.then(Box::new(identity::IdentityTokenProcessor) as Box<dyn TokenProcessor>)
    };
    let sub_processor = sub_processor
        .then(scroll::ScrollProcessor(Rc::clone(&bms), prompter))
        .then(section_len::SectionLenProcessor(Rc::clone(&bms), prompter))
        .then(speed::SpeedProcessor(Rc::clone(&bms), prompter))
        .then(sprite::SpriteProcessor(Rc::clone(&bms)))
        .then(stop::StopProcessor(Rc::clone(&bms), prompter))
        .then(text::TextProcessor(Rc::clone(&bms), prompter))
        .then(video::VideoProcessor(Rc::clone(&bms), prompter))
        .then(wav::WavProcessor::<P, T>(
            Rc::clone(&bms),
            prompter,
            PhantomData,
        ));
    random::RandomTokenProcessor::new(rng, sub_processor, false)
}

/// Returns commonly used processors.
pub fn common_preset<'a, P: Prompter, T: KeyLayoutMapper + 'a, R: Rng + 'a>(
    bms: Rc<RefCell<Bms>>,
    prompter: &'a P,
    rng: Rc<RefCell<R>>,
) -> impl TokenProcessor + 'a {
    let sub_processor = repr::RepresentationProcessor(Rc::clone(&bms))
        .then(bmp::BmpProcessor(Rc::clone(&bms), prompter))
        .then(bpm::BpmProcessor(Rc::clone(&bms), prompter))
        .then(judge::JudgeProcessor(Rc::clone(&bms), prompter))
        .then(metadata::MetadataProcessor(Rc::clone(&bms)))
        .then(music_info::MusicInfoProcessor(Rc::clone(&bms)))
        .then(scroll::ScrollProcessor(Rc::clone(&bms), prompter))
        .then(section_len::SectionLenProcessor(Rc::clone(&bms), prompter))
        .then(speed::SpeedProcessor(Rc::clone(&bms), prompter))
        .then(sprite::SpriteProcessor(Rc::clone(&bms)))
        .then(stop::StopProcessor(Rc::clone(&bms), prompter))
        .then(video::VideoProcessor(Rc::clone(&bms), prompter))
        .then(wav::WavProcessor::<P, T>(
            Rc::clone(&bms),
            prompter,
            PhantomData,
        ));
    random::RandomTokenProcessor::new(rng, sub_processor, true)
}

/// Returns all of processors this crate provided.
pub fn minor_preset<'a, P: Prompter, T: KeyLayoutMapper + 'a, R: Rng + 'a>(
    bms: Rc<RefCell<Bms>>,
    prompter: &'a P,
    rng: Rc<RefCell<R>>,
) -> impl TokenProcessor + 'a {
    let sub_processor = repr::RepresentationProcessor(Rc::clone(&bms))
        .then(bmp::BmpProcessor(Rc::clone(&bms), prompter))
        .then(bpm::BpmProcessor(Rc::clone(&bms), prompter))
        .then(judge::JudgeProcessor(Rc::clone(&bms), prompter))
        .then(metadata::MetadataProcessor(Rc::clone(&bms)))
        .then(music_info::MusicInfoProcessor(Rc::clone(&bms)));
    let sub_processor = if cfg!(feature = "minor-command") {
        sub_processor
            .then(Box::new(option::OptionProcessor(Rc::clone(&bms), prompter))
                as Box<dyn TokenProcessor>)
            .then(
                Box::new(resources::ResourcesProcessor(Rc::clone(&bms))) as Box<dyn TokenProcessor>
            )
    } else {
        sub_processor
            .then(Box::new(identity::IdentityTokenProcessor) as Box<dyn TokenProcessor>)
            .then(Box::new(identity::IdentityTokenProcessor) as Box<dyn TokenProcessor>)
    };
    let sub_processor = sub_processor
        .then(scroll::ScrollProcessor(Rc::clone(&bms), prompter))
        .then(section_len::SectionLenProcessor(Rc::clone(&bms), prompter))
        .then(speed::SpeedProcessor(Rc::clone(&bms), prompter))
        .then(sprite::SpriteProcessor(Rc::clone(&bms)))
        .then(stop::StopProcessor(Rc::clone(&bms), prompter))
        .then(text::TextProcessor(Rc::clone(&bms), prompter))
        .then(video::VideoProcessor(Rc::clone(&bms), prompter))
        .then(volume::VolumeProcessor(Rc::clone(&bms), prompter))
        .then(wav::WavProcessor::<P, T>(
            Rc::clone(&bms),
            prompter,
            PhantomData,
        ));
    random::RandomTokenProcessor::new(rng, sub_processor, true)
}

fn all_tokens<'a, F: FnMut(&'a Token<'_>) -> Result<Option<ParseWarning>, ParseError>>(
    input: &mut &'a [TokenWithRange<'_>],
    mut f: F,
) -> TokenProcessorResult {
    let mut warnings = vec![];
    for token in &**input {
        if let Some(warning) = f(token.content()).map_err(|err| err.into_wrapper(token))? {
            warnings.push(warning.into_wrapper(token));
        }
    }
    *input = &[];
    Ok(warnings)
}

fn all_tokens_with_range<
    'a,
    F: FnMut(&'a TokenWithRange<'_>) -> Result<Option<ParseWarning>, ParseError>,
>(
    input: &mut &'a [TokenWithRange<'_>],
    mut f: F,
) -> TokenProcessorResult {
    let mut warnings = vec![];
    for token in &**input {
        if let Some(warning) = f(token).map_err(|err| err.into_wrapper(token))? {
            warnings.push(warning.into_wrapper(token));
        }
    }
    *input = &[];
    Ok(warnings)
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
    F: FnMut(&str) -> Option<Result<T, ParseWarning>> + 'a,
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

        let mut buf = String::with_capacity(2);
        loop {
            // Get the next character pair, or end iteration if none remain
            let (c1, c2) = pairs_iter.next()?;
            buf.clear();
            buf.push(c1);
            buf.push(c2);

            // Store current pair index before incrementing
            let current_index = pair_index;
            pair_index += 1;

            // Try to parse the character pair using the provided parse_value function
            match parse_value(&buf) {
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
    case_sensitive_obj_id: bool,
    push_parse_warning: impl FnMut(ParseWarning) + 'a,
) -> impl Iterator<Item = (ObjTime, ObjId)> + 'a {
    parse_message_values_with_warnings(
        track,
        message,
        move |id| (id != "00").then(|| ObjId::try_from(id, case_sensitive_obj_id)),
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
        |digits| {
            Some(
                u8::from_str_radix(digits, 16).map_err(|_| {
                    ParseWarning::SyntaxError(format!("Invalid hex digits: {digits}"))
                }),
            )
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
