//! Definitions of the token in BMS format.

use std::borrow::Cow;

use crate::bms::command::{
    channel::{Channel, NoteChannelId},
    mixin::SourceRangeMixin,
    time::Track,
};

/// A token content of BMS format.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum Token<'a> {
    /// `#[name] [args]` Other command line starts from `#`.
    Header {
        /// String after `#` and until the first whitespace. It is always uppercase.
        name: Cow<'a, str>,
        /// String after `#name` and whitespaces.
        args: Cow<'a, str>,
    },
    /// Non-empty lines that not starts in `'#'` in bms file.
    NotACommand(&'a str),
    /// `#XXXYY:ZZ...`. Defines the message which places the object onto the score. `XXX` is the track, `YY` is the channel, and `ZZ...` is the object id sequence.
    Message {
        /// The track, or measure, must start from 1. But some player may allow the 0 measure (i.e. Lunatic Rave 2).
        track: Track,
        /// The channel commonly expresses what the lane be arranged the note to.
        channel: Channel,
        /// The message to the channel.
        message: Cow<'a, str>,
    },
}

/// A token with position information.
pub type TokenWithRange<'a> = SourceRangeMixin<Token<'a>>;

impl Token<'static> {
    /// Creates a [`Token::Header`] token with string literals.
    #[must_use]
    pub fn header(name: &'static str, args: &'static str) -> Self {
        Self::Header {
            name: name.into(),
            args: args.into(),
        }
    }
}

impl Token<'_> {
    /// Returns whether the token is a header with name `command_name`.
    #[must_use]
    pub fn is_header(&self, command_name: &str) -> bool {
        matches!(self,  Token::Header { name, .. } if name.eq_ignore_ascii_case(command_name))
    }

    /// Checks if a token is a control flow token.
    #[must_use]
    pub fn is_control_flow_token(&self) -> bool {
        [
            "RANDOM",
            "SETRANDOM",
            "IF",
            "ELSEIF",
            "ELSE",
            "ENDIF",
            "ENDRANDOM",
            "SWITCH",
            "SETSWITCH",
            "CASE",
            "DEF",
            "SKIP",
            "ENDSWITCH",
        ]
        .iter()
        .any(|command_name| self.is_header(command_name))
    }
}

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Header { name, args } => {
                if args.is_empty() {
                    write!(f, "#{name}")
                } else {
                    write!(f, "#{name} {args}")
                }
            }
            Token::NotACommand(comment) => write!(f, "{comment}"),
            Token::Message {
                track,
                channel,
                message,
            } => fmt_message(f, *track, *channel, message.as_ref()),
        }
    }
}

fn fmt_message(
    f: &mut std::fmt::Formatter<'_>,
    track: Track,
    channel: Channel,
    message: &str,
) -> std::fmt::Result {
    // Convert channel back to string representation using the new From trait
    let channel_id = NoteChannelId::from(channel);
    write!(f, "#{:03}{}:{}", track.0, channel_id, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_token(input: &'_ str) -> Token<'_> {
        let result = crate::bms::lex::TokenStream::parse_lex(input);
        assert_eq!(result.lex_warnings, vec![]);
        assert_eq!(result.tokens.iter().len(), 1);
        result.tokens.into_iter().next().unwrap().into_content()
    }

    #[test]
    fn test_display_roundtrip() {
        // Test basic commands
        let test_cases = vec![
            "#ARTIST Test Artist",
            "#TITLE Test Title",
            "#GENRE Test Genre",
            "#MAKER Test Maker",
            "#COMMENT Test Comment",
            "#PLAYLEVEL 5",
            "#RANK 2",
            "#TOTAL 100",
            "#PLAYER 1",
            "#DIFFICULTY 3",
            "#BASE 62",
            "#LNTYPE 1",
            "#LNTYPE 2",
            "#VOLWAV 100",
            "#BANNER banner.png",
            "#BACKBMP back.png",
            "#STAGEFILE stage.png",
            "#PATH_WAV /path/to/wav",
            "#VIDEOFILE video.mp4",
            "#MOVIE movie.mp4",
            "#PREVIEW preview.wav",
            "%EMAIL test@example.com",
            "%URL http://example.com",
            "#CHARSET UTF-8",
            "#DEFEXRANK 100",
            "#LNMODE 1",
            "#LNMODE 2",
            "#LNMODE 3",
            "#POORBGA 0",
            "#POORBGA 1",
            "#POORBGA 2",
        ];

        for input in test_cases {
            let token = parse_token(input);
            let output = format!("{}", token);
            assert_eq!(input, output, "Failed roundtrip for: {}", input);
        }
    }

    #[test]
    fn test_display_with_id_commands() {
        // Test commands with object IDs
        let test_cases = vec![
            "#WAV01 test.wav",
            "#BMP01 test.bmp",
            "#BMP00 poor.bmp",
            "#STOP01 48",
            "#TEXT01 Hello World",
            "#LNOBJ 01",
        ];

        for input in test_cases {
            let token = parse_token(input);
            let output = format!("{}", token);
            assert_eq!(input, output, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_display_control_flow() {
        // Test control flow commands
        let test_cases = vec![
            "#RANDOM 5",
            "#SETRANDOM 3",
            "#IF 2",
            "#ELSEIF 4",
            "#ELSE",
            "#ENDIF",
            "#ENDRANDOM",
            "#SWITCH 3",
            "#SETSWITCH 1",
            "#CASE 2",
            "#DEF",
            "#SKIP",
            "#ENDSW",
        ];

        for input in test_cases {
            let token = parse_token(input);
            let output = format!("{}", token);
            assert_eq!(input, output, "Failed roundtrip for: {}", input);
        }
    }

    #[test]
    fn test_display_message() {
        // Test message commands
        let test_cases = vec![
            "#00101:01020304",
            "#00204:01020304",
            "#00308:01020304",
            "#004SC:01020304",
            "#005SP:01020304",
        ];

        for input in test_cases {
            let token = parse_token(input);
            let output = format!("{}", token);
            assert_eq!(input, output, "Failed roundtrip for: {}", input);
        }
    }

    #[test]

    fn test_display_minor_commands() {
        // Test minor commands
        let test_cases = vec![
            "#OCT/FP",
            "#OPTION Test Option",
            "#MIDIFILE test.mid",
            "#CHARFILE test.chp",
            "#MATERIALS /path/to/materials",
            "#MATERIALSBMP materials.bmp",
            "#MATERIALSWAV materials.wav",
            "#DIVIDEPROP 192",
            "#CDDA 12345",
            "#VIDEOCOLORS 16",
        ];

        for input in test_cases {
            let token = parse_token(input);
            let output = format!("{}", token);
            assert_eq!(input, output, "Failed roundtrip for: {}", input);
        }
    }

    #[test]

    fn test_display_complex_commands() {
        // Test complex commands with multiple parameters
        let test_cases = vec![
            "#EXBMP01 255,0,0,0 exbmp.png",
            "#EXRANK01 2",
            "#EXWAV01 pvf 10000 -1000 48000 ex.wav",
            "#@BGA01 02 1 2 3 4 5 6",
            "#BGA01 02 1 2 3 4 5 6",
            "#CHANGEOPTION01 opt",
            "#ARGB01 255,255,255,255",
            "#STP 001.500 1500",
            "#WAVCMD 00 0E 61",
            "#SWBGA01 100:400:16:0:255,255,255,255 01020304",
            "#ExtChr 512 9 30 0 99 9",
            "#ExtChr 516 0 38 1 62 9 -2 -2",
            "#ExtChr 513 0 38 1 62 9 -2 -2 0 0",
        ];

        for input in test_cases {
            let token = parse_token(input);
            let output = format!("{}", token);
            assert_eq!(input, output, "Failed for: {}", input);
        }
    }
}
