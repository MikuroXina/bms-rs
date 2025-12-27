//! This module provides [`TokenProcessor`] and its implementations, which reads [`Token`] and applies data to [`Bms`].
//!
//! Also it provides a preset function [`full_preset`], that returns an opaque object of [`TokenProcessor`] consisted all of processors.

use std::{borrow::Cow, cell::RefCell, rc::Rc};

use itertools::Itertools;

use crate::bms::lex::TokenStream;
use crate::bms::{
    parse::{ParseError, ParseErrorWithRange, ParseWarningWithRange},
    prelude::*,
};
use crate::util::StrExtension;

mod bmp;
mod bpm;
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
#[derive(Debug, Clone, Copy)]
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

    /// Sets the current input view to the provided slice.
    pub const fn set_input(&mut self, view: &'a [&'t TokenWithRange<'t>]) {
        *self.input = view;
    }

    /// Returns a shared reference to the prompter.
    #[must_use]
    pub const fn prompter(&self) -> &'a P {
        self.prompter
    }

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
    ///
    /// # Errors
    ///
    /// Returns [`ParseErrorWithRange`] if `f` returns an error for any token.
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
    ///
    /// # Errors
    ///
    /// Returns [`ParseErrorWithRange`] when the token processor encounters a fatal parse error.
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

impl<T: TokenProcessor + ?Sized> TokenProcessor for Rc<T> {
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
        let checkpoint = ctx.save();
        let first_output = self.first.process(ctx)?;
        ctx.restore(checkpoint);
        let second_output = self.second.process(ctx)?;
        Ok((first_output, second_output))
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

/// Returns all of processors this crate provided.
pub(crate) fn full_preset<T: KeyLayoutMapper, R: Rng>(
    rng: Rc<RefCell<R>>,
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

    let bms_mapper = sub_processor.map(
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
            randomized: Default::default(),
        },
    );
    let bms_mapper = Rc::new(bms_mapper);

    random::RandomTokenProcessor::new(rng, bms_mapper).map(|(mut bms, randomized)| {
        bms.randomized = randomized;
        bms
    })
}

pub(crate) fn relax_tokens_default<'a>(tokens: &mut TokenStream<'a>) {
    for twr in tokens.tokens.iter_mut() {
        match twr.content_mut() {
            Token::Header { name, args } => {
                let n_ref = name.as_ref();
                let a_ref = args.as_ref();
                let mut new_name: Option<String> = None;
                let mut new_args: Option<String> = None;

                if n_ref.eq_ignore_ascii_case("RONDAM") {
                    new_name = Some("RANDOM".to_string());
                } else if n_ref.eq_ignore_ascii_case("END")
                    && a_ref.trim().eq_ignore_ascii_case("IF")
                {
                    new_name = Some("ENDIF".to_string());
                    new_args = Some(String::new());
                } else if a_ref.is_empty()
                    && let Some((kw, rest_trim)) = ["RANDOM", "IF"]
                        .iter()
                        .find_map(|kw| n_ref.strip_prefix_ignore_case(kw).map(|r| (*kw, r.trim())))
                {
                    let digits = if rest_trim.starts_with('[') && rest_trim.ends_with(']') {
                        &rest_trim[1..rest_trim.len() - 1]
                    } else {
                        rest_trim
                    };
                    if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                        new_name = Some(kw.to_string());
                        new_args = Some(digits.to_string());
                    }
                }

                if let Some(nn) = new_name {
                    *name = nn.into();
                }
                if let Some(na) = new_args {
                    *args = na.into();
                }
            }
            Token::NotACommand(line) => {
                if line.trim() == "＃ENDIF" {
                    *twr.content_mut() = Token::Header {
                        name: "ENDIF".to_string().into(),
                        args: String::new().into(),
                    };
                }
            }
            Token::Message { .. } => {}
        }
    }
}

/// A pre-parse transformer for lex tokens.
///
/// Implementations can rewrite headers, normalize arguments, or fix common typos.
/// The modifier runs before semantic token processors consume the stream.
pub trait TokenModifier {
    /// Apply in-place modifications to the provided token stream.
    ///
    /// Implementations should be deterministic and avoid altering source ranges.
    fn modify(&self, tokens: &mut TokenStream<'_>);

    /// Compose this modifier with another, applying `self` first and `second` after.
    ///
    /// The returned modifier preserves order and uses static dispatch.
    fn then<S>(self, second: S) -> SequentialTokenModifier<Self, S>
    where
        Self: Sized,
        S: TokenModifier,
    {
        SequentialTokenModifier {
            first: self,
            second,
        }
    }
}

/// A token modifier that sequentially applies two modifiers in order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SequentialTokenModifier<F, S> {
    first: F,
    second: S,
}

impl<F, S> TokenModifier for SequentialTokenModifier<F, S>
where
    F: TokenModifier,
    S: TokenModifier,
{
    fn modify(&self, tokens: &mut TokenStream<'_>) {
        self.first.modify(tokens);
        self.second.modify(tokens);
    }
}

/// A no-op token modifier used for strict parsing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoopTokenModifier;

impl TokenModifier for NoopTokenModifier {
    fn modify(&self, _tokens: &mut TokenStream<'_>) {}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
/// A relaxed modifier that normalizes common BMS typos and formats,
/// delegating to `rewrite_relaxed_tokens`.
pub struct DefaultTokenRelaxer;

impl TokenModifier for DefaultTokenRelaxer {
    fn modify(&self, tokens: &mut TokenStream<'_>) {
        relax_tokens_default(tokens);
    }
}

fn parse_obj_ids(
    track: Track,
    message: &SourceRangeMixin<&str>,
    case_sensitive_obj_id: &RefCell<bool>,
) -> (Vec<(ObjTime, ObjId)>, Vec<ParseWarningWithRange>) {
    let mut warnings = Vec::new();
    if !message.content().len().is_multiple_of(2) {
        warnings.push(
            ParseWarning::SyntaxError("expected 2-digit object ids".into()).into_wrapper(message),
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
                    warnings.push(warning.into_wrapper(message));
                    None
                }
            }
        });
    (messages.collect(), warnings)
}

fn parse_hex_values(
    track: Track,
    message: &SourceRangeMixin<&str>,
) -> (Vec<(ObjTime, u8)>, Vec<ParseWarningWithRange>) {
    let mut warnings = Vec::new();
    if !message.content().len().is_multiple_of(2) {
        warnings.push(
            ParseWarning::SyntaxError("expected 2-digit hex values".into()).into_wrapper(message),
        );
    }

    let denom = message.content().len() as u64 / 2;
    let parsed = message
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
                            .into_wrapper(message),
                    );
                    None
                },
                |value| ObjTime::new(track.0, i as u64, denom).map(|time| (time, value)),
            )
        });
    (parsed.collect(), warnings)
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
