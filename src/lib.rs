//! The BMS format parser.
//!
//! Be-Music Source, called BMS for short, is a file format devised by Urao Yane in 1998 for a simulator of the game Beatmania by KONAMI. This describes what and when notes are arranged and its music metadata. It is a plain text file with some "command" lines starting with `#` character.
//!
//! # Usage
//!
//! - **NOTE**: BMS files now is almost with Shift_JIS encoding. It's recommended to use [`encoding_rs`](https://crates.io/crates/encoding_rs) crate to parse raw file to `Cow<str>`, which is a compatible type of `&str`, using `AsRef::as_ref`.
//!
//! ## Simple Usage
//!
//! For most use cases, you can use the [`bms::parse_bms`] function to parse a BMS file in one step:
//!
//! ```rust
//! #[cfg(feature = "rand")]
//! use bms_rs::bms::{parse_bms, BmsOutput};
//! #[cfg(not(feature = "rand"))]
//! use bms_rs::bms::{ast::rng::RngMock, parse_bms_with_rng, BmsOutput};
//! #[cfg(not(feature = "rand"))]
//! use num::BigUint;
//! use bms_rs::bms::{command::channel::mapper::KeyLayoutBeat, BmsWarning};
//!
//! let source = std::fs::read_to_string("tests/bms/files/lilith_mx.bms").unwrap();
//! #[cfg(feature = "rand")]
//! let BmsOutput { bms, warnings }: BmsOutput = parse_bms::<KeyLayoutBeat>(&source).expect("must be parsed");
//! #[cfg(not(feature = "rand"))]
//! let BmsOutput { bms, warnings }: BmsOutput = parse_bms_with_rng::<KeyLayoutBeat, _>(&source, RngMock([BigUint::from(1u64)])).expect("must be parsed");
//! assert_eq!(warnings, vec![]);
//! println!("Title: {}", bms.header.title.as_deref().unwrap_or("Unknown"));
//! println!("BPM: {}", bms.arrangers.bpm.unwrap_or(120.into()));
//! ```
//!
//! ## Advanced Usage
//!
//! For more control over the parsing process, you can use the individual steps:
//!
//! At first, you can get the tokens stream with [`bms::lex::TokenStream::parse_lex`]. Then pass it and the random generator to [`bms::model::Bms::from_token_stream`] to get the notes data. Because BMS format has some randomized syntax.
//!
//! ```rust
//! #[cfg(feature = "rand")]
//! use rand::{rngs::StdRng, SeedableRng};
//! use bms_rs::bms::prelude::*;
//! use bms_rs::bms::command::channel::mapper::KeyLayoutBeat;
//! use num::BigUint;
//!
//! let source = std::fs::read_to_string("tests/bms/files/lilith_mx.bms").unwrap();
//! let LexOutput { tokens, lex_warnings } = TokenStream::parse_lex(&source);
//! assert_eq!(lex_warnings, vec![]);
//! // You can modify the tokens before parsing.
//!
//! #[cfg(feature = "rand")]
//! let ParseOutput { bms, parse_warnings }: ParseOutput = Bms::from_token_stream::<'_, KeyLayoutBeat, _, _>(
//!     &tokens, default_preset_with_rng(RandRng(StdRng::seed_from_u64(42))),
//! ).expect("must be parsed");
//! #[cfg(not(feature = "rand"))]
//! let ParseOutput { bms, parse_warnings }: ParseOutput = Bms::from_token_stream::<'_, KeyLayoutBeat, _, _>(
//!     &tokens, default_preset_with_rng(RngMock([BigUint::from(1u64)])),
//! ).expect("must be parsed");
//! // According to [BMS command memo#BEHAVIOR IN GENERAL IMPLEMENTATION](https://hitkey.bms.ms/cmds.htm#BEHAVIOR-IN-GENERAL-IMPLEMENTATION), the newer values are used for the duplicated objects.
//! assert_eq!(parse_warnings, vec![]);
//! let PlayingCheckOutput { playing_warnings, playing_errors } = bms.check_playing::<KeyLayoutBeat>();
//! assert_eq!(playing_warnings, vec![]);
//! assert_eq!(playing_errors, vec![]);
//! println!("Title: {}", bms.header.title.as_deref().unwrap_or("Unknown"));
//! println!("Artist: {}", bms.header.artist.as_deref().unwrap_or("Unknown"));
//! println!("BPM: {}", bms.arrangers.bpm.unwrap_or(120.into()));
//! ```
//!
//! # Features
//!
//! - For supported commands, see [docs.rs#Token](https://docs.rs/bms-rs/latest/bms_rs/bms/lex/token/enum.Token.html).
//!
//! - For supported note channels, see [docs.rs#Channel](https://docs.rs/bms-rs/latest/bms_rs/bms/command/channel/enum.Channel.html).
//!
//! ## Default Features
//!
//! - `bmson` feature enables the BMSON format support.
//! - `serde` feature enables the `serde` support. It supports [`serde::Serialize`] for all the definications in this crate, and [`serde::Deserialize`] for all the result types.
//! - `rand` feature enables the random number generator support. It supports [`bms::parse::random::rng::RandRng`].
//!
//! ## Optional Features
//!
//! - `minor-command` feature enables the commands that are almost never used in modern BMS Players.
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
#![warn(clippy::must_use_candidate)]
#![deny(rustdoc::broken_intra_doc_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod bms;
pub mod bmson;
pub mod chart_process;
pub mod diagnostics;

pub use bms::{command, lex, parse};
