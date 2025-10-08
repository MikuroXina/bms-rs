//! Lexical analyzer of BMS format.
//!
//! Raw [String] == [lex] ==> [`TokenStream`] (in [`BmsLexOutput`]) == [parse] ==> [Bms] (in
//! [`BmsParseOutput`])

pub mod cursor;
pub mod parser;
pub mod token;

use thiserror::Error;

use crate::{
    bms::command::mixin::SourceRangeMixin,
    diagnostics::{SimpleSource, ToAriadne},
};
use ariadne::{Color, Label, Report, ReportKind};

use self::{cursor::Cursor, parser::TokenParser, token::TokenWithRange};

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
    pub fn parse_lex(source: &'a str, parsers: Vec<Box<dyn TokenParser<'a>>>) -> LexOutput<'a> {
        let mut cursor = Cursor::new(source);

        let mut tokens = vec![];
        let mut warnings = vec![];
        while !cursor.is_end() {
            let command_range = cursor.save_checkpoint();

            // Try each parser in order
            let mut found_token = false;
            for parser in &parsers {
                let checkpoint = cursor.save_checkpoint();

                match parser.try_parse(&mut cursor) {
                    Ok(Some(token)) => {
                        let token_range = command_range.index..cursor.index();
                        tokens.push(SourceRangeMixin::new(token, token_range));
                        found_token = true;
                        break;
                    }
                    Ok(None) => {
                        cursor.restore_checkpoint(checkpoint);
                        continue;
                    }
                    Err(warning) => {
                        cursor.restore_checkpoint(checkpoint);
                        warnings.push(warning);
                        found_token = true;
                        break;
                    }
                }
            }

            if !found_token {
                warnings.push(cursor.make_err_expected_token("valid token"));
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
    use crate::{
        bms::{
            command::{channel::Channel, time::Track},
            lex::{LexOutput, TokenStream},
        },
        lex::token::Token,
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
        } = TokenStream::parse_lex(SRC, crate::bms::lex::parser::default_parsers());

        assert_eq!(warnings, vec![]);
        assert_eq!(
            tokens
                .tokens
                .iter()
                .map(|t| t.content().clone())
                .collect::<Vec<_>>(),
            vec![
                Token::header("PLAYER", "1"),
                Token::header("GENRE", "FUGA"),
                Token::header("TITLE", "BAR(^^)"),
                Token::header("ARTIST", "MikuroXina"),
                Token::header("BPM", "120"),
                Token::header("PLAYLEVEL", "6"),
                Token::header("RANK", "2"),
                Token::header("BACKBMP", "boon.jpg"),
                Token::header("WAV01", "hoge.WAV"),
                Token::header("WAV02", "foo.WAV"),
                Token::header("WAV03", "bar.WAV"),
                Token::Message {
                    track: Track(2),
                    channel: Channel::Note {
                        channel_id: "11".parse().unwrap(),
                    },
                    message: "0303030303".into(),
                },
                Token::Message {
                    track: Track(2),
                    channel: Channel::Note {
                        channel_id: "11".parse().unwrap(),
                    },
                    message: "0303000303".into(),
                },
                Token::Message {
                    track: Track(2),
                    channel: Channel::Note {
                        channel_id: "11".parse().unwrap(),
                    },
                    message: "010101".into(),
                },
                Token::Message {
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
