use std::{borrow::Cow, num::NonZeroU64};

use itertools::Itertools;

use super::{ParseWarning, Result};
use crate::bms::prelude::*;

mod bmp;
mod bpm;
mod judge;
mod metadata;
mod music_info;
mod option;
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
///
/// There are some invariants on calling:
///
/// - Once `on_message` is called, `one_header` must not be invoked after that.
/// - The effects of called `on_message` must be same regardless order of calls.
pub trait TokenProcessor {
    /// Processes a header command consists of `#{name} {args}`.
    fn on_header(&self, name: &str, args: &str) -> Result<()>;
    /// Processes a message command consists of `#{track}{channel}:{message}`.
    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()>;
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
