//! Lexical analyzer of BMS format.
//!
//! Raw [String] == [lex] ==> [`TokenStream`] (in [`BmsLexOutput`]) == [parse] ==> [Bms] (in
//! [`BmsParseOutput`])

mod command_impl;
mod cursor;
pub mod token;

use thiserror::Error;

use crate::{
    bms::command::mixin::{SourceRangeMixin, SourceRangeMixinExt},
    diagnostics::{SimpleSource, ToAriadne},
};
use ariadne::{Color, Label, Report, ReportKind};

use self::{
    cursor::Cursor,
    token::{Token, TokenWithRange},
};

/// An error occurred when lexical analysis.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LexWarning {
    /// The token was expected but not found.
    #[error("expected {message}, but not found")]
    ExpectedToken {
        /// What the expected is.
        message: String,
    },
    /// The channel was not recognized.
    #[error("channel `{channel}` not recognized")]
    UnknownChannel {
        /// The channel that was not recognized.
        channel: String,
    },
    /// The object id was not recognized.
    #[error("object `{object}` not recognized")]
    UnknownObject {
        /// The object id that was not recognized.
        object: String,
    },
    /// Failed to convert a byte into a base-62 character `0-9A-Za-z`.
    #[error("expected id format is base 62 (`0-9A-Za-z`)")]
    OutOfBase62,
    /// An unknown command was encountered.
    #[error("unknown command `{command}`")]
    UnknownCommand {
        /// The unknown command that was encountered.
        command: String,
    },
}

/// A [`LexWarning`] type with position information.
pub type LexWarningWithRange = SourceRangeMixin<LexWarning>;

/// Type alias of `core::result::Result<T, LexWarningWithRange>`
pub(crate) type Result<T> = core::result::Result<T, LexWarningWithRange>;

/// Lex Parsing Results, includes tokens and warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[must_use]
pub struct LexOutput<'a> {
    /// tokens
    pub tokens: TokenStream<'a>,
    /// warnings
    pub lex_warnings: Vec<LexWarningWithRange>,
}

/// A list of tokens.
/// This is a wrapper of [`Vec<TokenWithRange<'a>>`] that provides some additional methods.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TokenStream<'a> {
    /// The tokens.
    pub tokens: Vec<TokenWithRange<'a>>,
}

impl<'a> IntoIterator for TokenStream<'a> {
    type Item = TokenWithRange<'a>;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.tokens.into_iter()
    }
}

impl<'a> IntoIterator for &'a TokenStream<'a> {
    type Item = &'a TokenWithRange<'a>;
    type IntoIter = std::slice::Iter<'a, TokenWithRange<'a>>;
    fn into_iter(self) -> Self::IntoIter {
        self.tokens.iter()
    }
}

/// A list of tokens reference.
/// This is a wrapper of [`Vec<&'a TokenWithRange<'a>>`] that provides some additional methods.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TokenRefStream<'a> {
    /// The tokens.
    pub token_refs: Vec<&'a TokenWithRange<'a>>,
}

impl<'a> IntoIterator for TokenRefStream<'a> {
    type Item = &'a TokenWithRange<'a>;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.token_refs.into_iter()
    }
}

impl<'a, 'b> IntoIterator for &'b TokenRefStream<'a> {
    type Item = &'b &'a TokenWithRange<'a>;
    type IntoIter = std::slice::Iter<'b, &'a TokenWithRange<'a>>;
    fn into_iter(self) -> Self::IntoIter {
        self.token_refs.iter()
    }
}

impl<'a> TokenStream<'a> {
    /// Analyzes and converts the BMS format text into [`TokenStream`].
    /// Use this function when you want to parse the BMS format text with a custom channel parser.
    pub fn parse_lex(source: &'a str) -> LexOutput<'a> {
        let mut cursor = Cursor::new(source);

        let mut tokens = vec![];
        let mut warnings = vec![];
        while !cursor.is_end() {
            match Token::parse(&mut cursor) {
                Ok(token_with_range) => {
                    // If the token is UnknownCommand, also add a warning
                    if let Token::UnknownCommand(cmd) = token_with_range.content() {
                        warnings.push(
                            LexWarning::UnknownCommand {
                                command: cmd.to_string(),
                            }
                            .into_wrapper(&token_with_range),
                        );
                    }

                    tokens.push(token_with_range);
                }
                Err(warning) => {
                    warnings.push(warning);
                }
            }
        }

        let case_sensitive = tokens
            .iter()
            .any(|token| matches!(token.content(), Token::Base62));
        if !case_sensitive {
            for token in &mut tokens {
                token.content_mut().make_id_uppercase();
            }
        }
        LexOutput {
            tokens: TokenStream { tokens },
            lex_warnings: warnings,
        }
    }

    /// Makes a new iterator of tokens.
    pub fn iter(&self) -> std::slice::Iter<'_, TokenWithRange<'a>> {
        self.tokens.iter()
    }
}

impl<'a> TokenRefStream<'a> {
    /// Makes a new iterator of token references.
    pub fn iter(&self) -> std::slice::Iter<'_, &'a TokenWithRange<'a>> {
        self.token_refs.iter()
    }
}

impl ToAriadne for LexWarningWithRange {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        let (start, end) = self.as_span();
        let filename = src.name().to_string();
        Report::build(ReportKind::Warning, (filename.clone(), start..end))
            .with_message("lex: ".to_string() + &self.content().to_string())
            .with_label(Label::new((filename, start..end)).with_color(Color::Yellow))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, str::FromStr};

    use fraction::{GenericDecimal, GenericFraction};

    use crate::bms::{
        command::{JudgeLevel, PlayerMode, channel::Channel, time::Track},
        lex::{LexOutput, TokenStream},
    };

    #[test]
    fn simple() {
        const SRC: &str = r"
#PLAYER 1
#GENRE FUGA
#TITLE BAR(^^)
#ARTIST MikuroXina
#BPM 120
#PLAYLEVEL 6
#RANK 2
#BACKBMP boon.jpg

#WAV01 hoge.WAV
#WAV02 foo.WAV
#WAV03 bar.WAV

#00211:0303030303

#00211:0303000303

#00211:010101
#00211:00020202
";

        let LexOutput {
            tokens,
            lex_warnings: warnings,
        } = TokenStream::parse_lex(SRC);

        assert_eq!(warnings, vec![]);
        assert_eq!(
            tokens
                .tokens
                .iter()
                .cloned()
                .map(|t| t.content().clone())
                .collect::<Vec<_>>(),
            vec![
                Player(PlayerMode::Single),
                Genre("FUGA"),
                Title("BAR(^^)"),
                Artist("MikuroXina"),
                Bpm(GenericDecimal::from_fraction(
                    GenericFraction::from_str("120").unwrap()
                )),
                PlayLevel(6),
                Rank(JudgeLevel::Normal),
                BackBmp(Path::new("boon.jpg")),
                Wav("01".try_into().unwrap(), Path::new("hoge.WAV")),
                Wav("02".try_into().unwrap(), Path::new("foo.WAV")),
                Wav("03".try_into().unwrap(), Path::new("bar.WAV")),
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        channel_id: "11".parse().unwrap(),
                    },
                    message: "0303030303".into(),
                },
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        channel_id: "11".parse().unwrap(),
                    },
                    message: "0303000303".into(),
                },
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        channel_id: "11".parse().unwrap(),
                    },
                    message: "010101".into(),
                },
                Message {
                    track: Track(2),
                    channel: Channel::Note {
                        channel_id: "11".parse().unwrap(),
                    },
                    message: "00020202".into(),
                },
            ]
        );
    }
}
