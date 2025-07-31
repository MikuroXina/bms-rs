//! The BMS format parser.
//!
//! Be-Music Source, called BMS for short, is a file format devised by Urao Yane in 1998 for a simulator of the game Beatmania by KONAMI. This describes what and when notes are arranged and its music metadata. It is a plain text file with some "command" lines starting with `#` character.
//!
//! # Usage
//!
//! At first, you can get the tokens stream with [`lex::parse`]. Then pass it and the random generator to [`parse::Bms::from_token_stream`] to get the notes data. Because BMS format has some randomized syntax.
//!
//! ```
//! use rand::{rngs::StdRng, SeedableRng};
//! use bms_rs::bms::{
//!     lex::{parse, parse_with_channel_parser, BmsLexOutput},
//!     parse::{prompt::AlwaysWarn, random::rng::RandRng, model::Bms, BmsParseOutput},
//!     command::channel::read_channel_beat
//! };
//!
//! let source = std::fs::read_to_string("tests/files/lilith_mx.bms").unwrap();
//! let BmsLexOutput { tokens: _, lex_warnings } = parse(&source);
//! assert_eq!(lex_warnings, vec![]);
//! // Or you can use another preset.
//! // This crate defines some presets for Beat(5K/7K/10K/14K) and Pop'n(5K/9K/18K) modes.
//! // See `bms::lex::command::channel` documentation for the pre-defined channel parsers.
//! // Please see [BMS command memo](https://hitkey.bms.ms/cmds.htm#KEYMAP-TABLE) for more details.
//! let BmsLexOutput { tokens, lex_warnings } = parse_with_channel_parser(&source, &read_channel_beat);
//! assert_eq!(lex_warnings, vec![]);
//! // You can modify the tokens before parsing, for some commands that this library does not warpped.
//! let rng = RandRng(StdRng::seed_from_u64(42));
//! let BmsParseOutput { bms, parse_warnings, playing_warnings, playing_errors } = Bms::from_token_stream(
//!     &tokens, rng, AlwaysWarn
//!     );
//! assert_eq!(parse_warnings, vec![]);
//! assert_eq!(playing_warnings, vec![]);
//! assert_eq!(playing_errors, vec![]);
//! ```
//!
//! # Features
//!
//! - `bmson` feature enables the BMSON format support.
//! - `serde` feature enables the `serde` support. It supports `Serialize` for all the definications in this crate, and `Deserialize` for all the result types.
//! - `rand` feature enables the random number generator support. It supports [`RandRng`].
//!
//! # About the format
//!
//! ## Command
//!
//! Each command starts with `#` character, and other lines will be ignored. Some commands require arguments separated by whitespace character such as spaces or tabs.
//!
//! ```text
//! #PLAYER 1
//! #GENRE FUGA
//! #TITLE BAR(^^)
//! #ARTIST MikuroXina
//! #BPM 120
//! #PLAYLEVEL 6
//! #RANK 2
//!
//! #WAV01 hoge.WAV
//! #WAV02 foo.WAV
//! #WAV03 bar.WAV
//!
//! #00211:0303030303
//! ```
//!
//! ### Header command
//!
//! Header commands are used to express the metadata of the music or the definition for note arrangement.
//!
//! ### Message command
//!
//! Message command starts with `#XXXYY:ZZ...`. `XXX` is the number of the measure, `YY` is the channel of the message, and `ZZ...` is the object id sequence.
//!
//! The measure must start from 1, but some player may allow the 0 measure (i.e. Lunatic Rave 2).
//!
//! The channel commonly expresses what the lane be arranged the note to.
//!
//! The object id is formed by 2-digit of 36-radix (`[0-9a-zA-Z]`) integer. So the sequence length must be an even number. The `00` object id is the special id, expresses the rest (no object lies). The object lies on the position divided equally by how many the object is in the measure. For example:
//!
//! ```text
//! #00211:0303000303
//! ```
//!
//! This will be placed as:
//!
//! ```text
//! 003|--|--------------|
//!    |  |03            |
//!    |  |03            |
//!    |  |              |
//!    |  |03            |
//! 002|--|03------------|
//!    |  |  []  []  []  |
//!    |()|[]  []  []  []|
//!    |-----------------|
//! ```

#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod bms;
#[cfg(feature = "bmson")]
#[cfg_attr(docsrs, doc(cfg(feature = "bmson")))]
pub mod bmson;

pub use bms::{command, lex, parse};
